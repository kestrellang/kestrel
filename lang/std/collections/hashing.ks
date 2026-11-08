// Hashers: `DefaultHasher` (keyed SipHash-1-3, used by `Dictionary` and
// `Set`) and `FastHasher` (an unkeyed word-at-a-time mixer).

module std.collections

import std.core.(Hasher, Hashable, Defaultable)
import std.numeric.(UInt8, UInt64, Int64, DefaultRandom)
import std.collections.(Slice)
import std.memory.(RawPointer)

// ============================================================================
// DEFAULT HASHER (SIPHASH-1-3)
// ============================================================================

/// The process-wide SipHash key, drawn from `DefaultRandom` the first time
/// a `DefaultHasher()` is made. One key per process (as in Swift), so every
/// table agrees on every hash within a run, and a table's iteration order
/// changes from run to run. Setting `KESTREL_DETERMINISTIC_HASHING` in the
/// environment pins the key to zero for reproducible runs (tests, diffs).
var sipProcessKey0: UInt64 = 0;
var sipProcessKey1: UInt64 = 0;
var sipProcessKeyed: Bool = false;

@extern(.C, mangleName: "getenv")
func hashing_getenv(consuming name: RawPointer) -> RawPointer

/// Draws the process key once.
func sipProcessKey() -> (UInt64, UInt64) {
    if not sipProcessKeyed {
        sipProcessKeyed = true;
        let name = "KESTREL_DETERMINISTIC_HASHING".toCString();
        let value = unsafe hashing_getenv(name.raw.asRaw());
        let pinned = not value.isNull;
        name.free();
        if not pinned {
            var rng = DefaultRandom();
            sipProcessKey0 = rng.nextUInt64();
            sipProcessKey1 = rng.nextUInt64()
        }
    }
    (sipProcessKey0, sipProcessKey1)
}

/// SipHash-1-3 (Aumasson & Bernstein): a keyed hash, so without the key an
/// attacker can't predict which inputs collide. This is what Rust and Swift
/// hash tables use by default, and it's the `Hasher` behind `Dictionary` and
/// `Set` unless they name another.
///
/// `DefaultHasher()` uses a random per-process key; `DefaultHasher(key0:key1:)`
/// takes an explicit 128-bit key and gives the reference SipHash-1-3 value
/// of the bytes written. Like `FastHasher` it streams (split writes hash
/// the same as one write) and integers go through `write(uint64:)` as one
/// word. It's slower than `FastHasher`; pick that for trusted keys when
/// hashing is the bottleneck.
///
/// # Examples
///
/// ```
/// var h = DefaultHasher();
/// "hello".hash(into: h);
/// let hash = h.finish();  // stable within a run, varies between runs
/// ```
///
/// # Representation
///
/// The four SipHash state words, up to 7 pending bytes (`tail`,
/// little-endian) and the total byte count.
public struct DefaultHasher: Hasher, Defaultable {
    private var v0: UInt64
    private var v1: UInt64
    private var v2: UInt64
    private var v3: UInt64
    private var tail: UInt64
    private var tailCount: Int64
    private var total: Int64

    /// @name Default
    /// A hasher keyed with this process's random key.
    public init() {
        let (k0, k1) = sipProcessKey();
        self.init(key0: k0, key1: k1)
    }

    /// A hasher with an explicit 128-bit key (`key0` is its first 8 bytes,
    /// little-endian).
    public init(key0 k0: UInt64, key1 k1: UInt64) {
        self.v0 = k0.bitwiseXor(8317987319222330741);   // "somepseu"
        self.v1 = k1.bitwiseXor(7237128888997146477);   // "dorandom"
        self.v2 = k0.bitwiseXor(7816392313619706465);   // "lygenera"
        self.v3 = k1.bitwiseXor(8387220255154660723);   // "tedbytes"
        self.tail = 0;
        self.tailCount = 0;
        self.total = 0
    }

    /// Tops up the pending word, compresses whole 8-byte little-endian
    /// words straight from `bytes`, and keeps the remainder pending.
    public mutating func write(bytes: Slice[UInt8]) {
        let count = bytes.count;
        let ptr = bytes.pointer;
        self.total = self.total + count;
        var i: Int64 = 0;
        while self.tailCount > 0 and i < count {
            self.pushByte(unsafe ptr.offset(by: i).read());
            i = i + 1
        }
        while i + 8 <= count {
            var word: UInt64 = 0;
            var k: Int64 = 7;
            while k >= 0 {
                word = word.shiftLeft(by: 8).bitwiseOr(UInt64(from: unsafe ptr.offset(by: i + k).read()));
                k = k - 1
            }
            self.compress(word);
            i = i + 8
        }
        while i < count {
            self.pushByte(unsafe ptr.offset(by: i).read());
            i = i + 1
        }
    }

    /// The same as writing the value's 8 little-endian bytes: one
    /// compression when no partial word is pending.
    public mutating func write(uint64 value: UInt64) {
        self.total = self.total + 8;
        if self.tailCount == 0 {
            self.compress(value);
            return
        }
        let shift = self.tailCount * 8;
        self.compress(self.tail.bitwiseOr(value.shiftLeft(by: shift)));
        self.tail = value.shiftRight(by: 64 - shift)
    }

    /// The last block (pending bytes, with the length's low byte on top),
    /// then three finalization rounds.
    public mutating func finish() -> UInt64 {
        let b = self.tail.bitwiseOr(UInt64(truncating: self.total).shiftLeft(by: 56));
        self.compress(b);
        self.v2 = self.v2.bitwiseXor(255);
        self.round();
        self.round();
        self.round();
        self.v0.bitwiseXor(self.v1).bitwiseXor(self.v2).bitwiseXor(self.v3)
    }

    /// One message word: `v3 ^= m`, one SipRound (the "1" in 1-3), `v0 ^= m`.
    private mutating func compress(word: UInt64) {
        self.v3 = self.v3.bitwiseXor(word);
        self.round();
        self.v0 = self.v0.bitwiseXor(word)
    }

    private mutating func pushByte(byte: UInt8) {
        let shift = self.tailCount * 8;
        self.tail = self.tail.bitwiseOr(UInt64(from: byte).shiftLeft(by: shift));
        self.tailCount = self.tailCount + 1;
        if self.tailCount == 8 {
            self.compress(self.tail);
            self.tail = 0;
            self.tailCount = 0
        }
    }

    private mutating func round() {
        self.v0 = self.v0.addWrapping(self.v1);
        self.v1 = self.v1.rotateLeft(by: 13).bitwiseXor(self.v0);
        self.v0 = self.v0.rotateLeft(by: 32);
        self.v2 = self.v2.addWrapping(self.v3);
        self.v3 = self.v3.rotateLeft(by: 16).bitwiseXor(self.v2);
        self.v0 = self.v0.addWrapping(self.v3);
        self.v3 = self.v3.rotateLeft(by: 21).bitwiseXor(self.v0);
        self.v2 = self.v2.addWrapping(self.v1);
        self.v1 = self.v1.rotateLeft(by: 17).bitwiseXor(self.v2);
        self.v2 = self.v2.rotateLeft(by: 32)
    }
}

// ============================================================================
// FAST HASHER
// ============================================================================

/// A fast, unkeyed `Hasher`: a word-at-a-time multiply-xorshift mixer
/// with Murmur3's `fmix64` finalizer.
///
/// Input is folded in 8 bytes per round, and integers (through
/// `write(uint64:)`) take a single round. It streams: bytes are buffered
/// into whole words, so split writes hash the same as one write.
/// `finish()` folds in the total byte count and runs `fmix64`, so every
/// input bit avalanches across the output. Variable-length values end
/// their own input (`String` writes its length), which keeps `["ab", "c"]`
/// and `["a", "bc"]` apart.
///
/// **Not adversarially safe.** It's unkeyed, so anyone who chooses the
/// keys can craft collisions, but hashes and iteration order are the same
/// on every run. Pick it for trusted keys where speed matters (internal
/// IDs, parser symbols): `Dictionary[K, V, FastHasher]`. The default,
/// `DefaultHasher`, is keyed SipHash.
///
/// # Examples
///
/// ```
/// var h = FastHasher();
/// "hello".hash(into: h);
/// let hash = h.finish();  // the same on every run
///
/// let ids: Dictionary[Int64, String, FastHasher] = Dictionary();
/// ```
///
/// # Representation
///
/// `state` (the running digest), `pending` and `pendingCount` (up to 7
/// bytes not yet forming a word, little-endian), and `total` (bytes seen).
public struct FastHasher: Hasher, Defaultable {
    private var state: UInt64
    private var pending: UInt64
    private var pendingCount: Int64
    private var total: Int64

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Default
    /// A hasher seeded with the golden-ratio constant SplitMix64 uses.
    public init() {
        self.state = 11400714819323198485;  // 0x9e3779b97f4a7c15
        self.pending = 0;
        self.pendingCount = 0;
        self.total = 0;
    }

    // ========================================================================
    // HASHER PROTOCOL
    // ========================================================================

    /// Tops up the pending partial word, mixes whole 8-byte little-endian
    /// words straight from `bytes`, and keeps the remainder pending.
    public mutating func write(bytes: Slice[UInt8]) {
        let count = bytes.count;
        let ptr = bytes.pointer;
        self.total = self.total + count;
        var i: Int64 = 0;
        // Finish a partial word left by an earlier write.
        while self.pendingCount > 0 and i < count {
            self.pushByte(unsafe ptr.offset(by: i).read());
            i = i + 1
        }
        while i + 8 <= count {
            // Byte-wise assembly is alignment-safe; LLVM folds it into a
            // single load where the target allows.
            var word: UInt64 = 0;
            var k: Int64 = 7;
            while k >= 0 {
                word = word.shiftLeft(by: 8).bitwiseOr(UInt64(from: unsafe ptr.offset(by: i + k).read()));
                k = k - 1
            }
            self.mix(word);
            i = i + 8
        }
        while i < count {
            self.pushByte(unsafe ptr.offset(by: i).read());
            i = i + 1
        }
    }

    /// The same as writing the value's 8 little-endian bytes. With no
    /// partial word pending (the usual case) that's a single round.
    public mutating func write(uint64 value: UInt64) {
        self.total = self.total + 8;
        if self.pendingCount == 0 {
            self.mix(value);
            return
        }
        let shift = self.pendingCount * 8;
        self.mix(self.pending.bitwiseOr(value.shiftLeft(by: shift)));
        self.pending = value.shiftRight(by: 64 - shift)
    }

    /// Murmur3's `fmix64` over the state.
    public mutating func finish() -> UInt64 {
        let m1 = 18397679294719823053;  // 0xff51afd7ed558ccd
        let m2 = 14181476777654086739;  // 0xc4ceb9fe1a85ec53
        if self.pendingCount > 0 {
            self.mix(self.pending);
            self.pending = 0;
            self.pendingCount = 0
        }
        // The byte count separates inputs that differ only in trailing zeros.
        self.mix(UInt64(truncating: self.total));
        var x = self.state;
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiplyWrapping(m1);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiplyWrapping(m2);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        self.state = x;
        x
    }

    /// `state = (state ^ word) * K`, then fold the high half down: the
    /// multiply only carries bits upward, so the shift brings them back
    /// into the low bits the table index uses.
    // Appends one byte to the pending word, mixing it once it's full.
    private mutating func pushByte(byte: UInt8) {
        let shift = self.pendingCount * 8;
        self.pending = self.pending.bitwiseOr(UInt64(from: byte).shiftLeft(by: shift));
        self.pendingCount = self.pendingCount + 1;
        if self.pendingCount == 8 {
            self.mix(self.pending);
            self.pending = 0;
            self.pendingCount = 0
        }
    }

    private mutating func mix(word: UInt64) {
        let k = 11400714819323198485;  // 0x9e3779b97f4a7c15, odd
        var x = self.state.bitwiseXor(word);
        x = x.multiplyWrapping(k);
        self.state = x.bitwiseXor(x.shiftRight(by: 29))
    }
}
