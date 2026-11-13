// Hashers: `DefaultHasher` (keyed SipHash-1-3, used by `Dictionary` and
// `Set`) and `FoldHasher` (foldhash, unkeyed).

module std.collections

import std.core.(Hasher, Hashable, Defaultable)
import std.numeric.(UInt8, UInt64, Int64, DefaultRandom)
import std.collections.(Slice)
import std.memory.(Pointer)

// ============================================================================
// DEFAULT HASHER (SIPHASH-1-3)
// ============================================================================

/// The process-wide SipHash key, drawn from `DefaultRandom` the first time
/// a `DefaultHasher()` is made. One key per process (as in Swift), so every
/// table agrees on every hash within a run, and a table's iteration order
/// changes from run to run. For a fixed order, name a hasher with a fixed
/// key in the table's type, e.g. `Dictionary[K, V, FastHasher]`.
var sipProcessKey0: UInt64 = 0;
var sipProcessKey1: UInt64 = 0;
var sipProcessKeyed: Bool = false;

/// Draws the process key once.
func sipProcessKey() -> (UInt64, UInt64) {
    if not sipProcessKeyed {
        sipProcessKeyed = true;
        var rng = DefaultRandom();
        sipProcessKey0 = rng.nextUInt64();
        sipProcessKey1 = rng.nextUInt64()
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
/// of the bytes written. It streams (split writes hash the same as one
/// write) and integers go through `write(uint64:)` as one word. It's
/// slower than `FastHasher`; pick that for trusted keys when hashing is
/// the bottleneck.
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
// FAST HASHER (FOLDHASH)
// ============================================================================

/// The fast, unkeyed `Hasher`: currently `FoldHasher`. Name this for
/// "fast, not attack-resistant"; name `FoldHasher` to pin the algorithm.
public type FastHasher = FoldHasher;

// foldhash's fixed seeds (hex digits of pi): ARBITRARY4..7 are the fold
// and expand seeds; ARBITRARY3 starts the accumulator (see `init`).
let foldSeed: UInt64 = 4983270260364809079;      // 0x452821e638d01377
let foldExpandSeed: UInt64 = 13714699805381954668;   // 0xbe5466cf34e90c6c
let foldExpandSeed2: UInt64 = 13883517620612518109;  // 0xc0ac29b7c97c50dd
let foldExpandSeed3: UInt64 = 4577018097722394903;   // 0x3f84d5b5b5470917

/// foldhash's fast variant (Orson Peters, the default in Rust's
/// `hashbrown`), with its fixed seeds: a fast, unkeyed `Hasher` built on
/// the folded multiply (the 128-bit product of two words, its halves
/// XORed together). It passes SMHasher and gives the same values as the
/// Rust crate's `foldhash::fast::FixedState`.
///
/// Each `write(bytes:)` is absorbed as one block, so unlike `DefaultHasher`
/// splitting a write changes the hash; variable-length values end their
/// own input (`String` writes its length) to keep `["ab", "c"]` and
/// `["a", "bc"]` apart. Integers collect in a 128-bit buffer and are
/// folded in two at a time.
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
/// var h = FoldHasher();
/// "hello".hash(into: h);
/// let hash = h.finish();  // the same on every run
///
/// let ids: Dictionary[Int64, String, FoldHasher] = Dictionary();
/// ```
///
/// # Representation
///
/// `accumulator` (the running digest) and the integer buffer: `spongeLo`
/// and `spongeHi` hold up to two pending words, `spongeLen` the bits used
/// (0, 64 or 128).
public struct FoldHasher: Hasher, Defaultable {
    private var accumulator: UInt64
    private var spongeLo: UInt64
    private var spongeHi: UInt64
    private var spongeLen: Int64

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Default
    /// A hasher with foldhash's fixed seed.
    public init() {
        self.accumulator = 589684135938649225;  // 0x082efa98ec4e6c89
        self.spongeLo = 0;
        self.spongeHi = 0;
        self.spongeLen = 0;
    }

    // ========================================================================
    // HASHER PROTOCOL
    // ========================================================================

    /// Absorbs `bytes` as one block. Up to 16 bytes take one folded
    /// multiply; longer inputs fold 16 or 64 bytes per step.
    public mutating func write(bytes: Slice[UInt8]) {
        let len = bytes.count;
        let ptr = bytes.pointer;
        let base = self.accumulator.rotateRight(by: len % 64);
        if len > 16 {
            let s1 = base.addWrapping(foldExpandSeed);
            if len < 256 {
                self.accumulator = foldMedium(ptr, 0, len, base, s1);
                return
            }
            self.accumulator = foldLong(ptr, len, base, s1);
            return
        }
        var s0 = base;
        var s1 = foldExpandSeed;
        if len >= 8 {
            s0 = s0.bitwiseXor(foldLoadWord(ptr, 0, 8));
            s1 = s1.bitwiseXor(foldLoadWord(ptr, len - 8, 8))
        } else if len >= 4 {
            s0 = s0.bitwiseXor(foldLoadWord(ptr, 0, 4));
            s1 = s1.bitwiseXor(foldLoadWord(ptr, len - 4, 4))
        } else if len > 0 {
            s0 = s0.bitwiseXor(foldLoadWord(ptr, 0, 1));
            let mid = foldLoadWord(ptr, len / 2, 1);
            let hi = foldLoadWord(ptr, len - 1, 1);
            s1 = s1.bitwiseXor(hi.shiftLeft(by: 8).bitwiseOr(mid))
        }
        self.accumulator = foldedMultiply(s0, s1)
    }

    /// Buffers the word; once two are pending they're folded in together.
    public mutating func write(uint64 value: UInt64) {
        if self.spongeLen == 0 {
            self.spongeLo = value;
            self.spongeLen = 64;
            return
        }
        if self.spongeLen == 64 {
            self.spongeHi = value;
            self.spongeLen = 128;
            return
        }
        self.foldSponge();
        self.spongeLo = value;
        self.spongeHi = 0;
        self.spongeLen = 64
    }

    /// Folds in any buffered words; otherwise the accumulator as is.
    public mutating func finish() -> UInt64 {
        if self.spongeLen > 0 {
            self.foldSponge();
            self.spongeLo = 0;
            self.spongeHi = 0;
            self.spongeLen = 0
        }
        self.accumulator
    }

    private mutating func foldSponge() {
        self.accumulator = foldedMultiply(
            self.spongeLo.bitwiseXor(self.accumulator),
            self.spongeHi.bitwiseXor(foldSeed))
    }
}

/// The 128-bit product `x * y` with its two halves XORed: foldhash's
/// mixing step, one widening multiply.
func foldedMultiply(x: UInt64, y: UInt64) -> UInt64 {
    let (low, high) = x.multiplyFull(y);
    low.bitwiseXor(high)
}

/// `width` (1, 4 or 8) little-endian bytes at `ptr + at`. Byte-wise
/// assembly is alignment-safe; LLVM folds it into one load.
func foldLoadWord(ptr: Pointer[UInt8], at: Int64, width: Int64) -> UInt64 {
    var word: UInt64 = 0;
    var k = width - 1;
    while k >= 0 {
        word = word.shiftLeft(by: 8).bitwiseOr(UInt64(from: unsafe ptr.offset(by: at + k).read()));
        k = k - 1
    }
    word
}

/// foldhash's `hash_bytes_medium` over `ptr[start..<start + len]`
/// (16 <= len): 16-byte chunks from the front pair up with 16-byte chunks
/// from the back until they meet.
func foldMedium(ptr: Pointer[UInt8], start: Int64, len: Int64, s0In: UInt64, s1In: UInt64) -> UInt64 {
    var s0 = s0In;
    var s1 = s1In;
    var front: Int64 = 0;
    while front + 16 <= len {
        let backEnd = len - front;
        if front >= backEnd { break };
        let back = backEnd - 16;
        let a = foldLoadWord(ptr, start + front, 8);
        let b = foldLoadWord(ptr, start + front + 8, 8);
        let c = foldLoadWord(ptr, start + back, 8);
        let d = foldLoadWord(ptr, start + back + 8, 8);
        s0 = foldedMultiply(a.bitwiseXor(s0), c.bitwiseXor(foldSeed));
        s1 = foldedMultiply(b.bitwiseXor(s1), d.bitwiseXor(foldSeed));
        front = front + 16
    }
    s0.bitwiseXor(s1)
}

/// foldhash's `hash_bytes_long` (256 <= len): four lanes over 64-byte
/// chunks, then the tail (at least its last 16 bytes) through `foldMedium`.
func foldLong(ptr: Pointer[UInt8], len: Int64, base: UInt64, s1In: UInt64) -> UInt64 {
    var s0 = base;
    var s1 = s1In;
    var s2 = base.addWrapping(foldExpandSeed2);
    var s3 = base.addWrapping(foldExpandSeed3);
    let fold = foldSeed;
    var i: Int64 = 0;
    while i + 64 <= len {
        s0 = foldedMultiply(foldLoadWord(ptr, i, 8).bitwiseXor(s0), foldLoadWord(ptr, i + 32, 8).bitwiseXor(fold));
        s1 = foldedMultiply(foldLoadWord(ptr, i + 8, 8).bitwiseXor(s1), foldLoadWord(ptr, i + 40, 8).bitwiseXor(fold));
        s2 = foldedMultiply(foldLoadWord(ptr, i + 16, 8).bitwiseXor(s2), foldLoadWord(ptr, i + 48, 8).bitwiseXor(fold));
        s3 = foldedMultiply(foldLoadWord(ptr, i + 24, 8).bitwiseXor(s3), foldLoadWord(ptr, i + 56, 8).bitwiseXor(fold));
        i = i + 64
    }
    s0 = s0.bitwiseXor(s2);
    s1 = s1.bitwiseXor(s3);
    let remainder = len - i;
    if remainder == 0 { return s0.bitwiseXor(s1) };
    let tail = if remainder > 16 { remainder } else { 16 };
    foldMedium(ptr, len - tail, tail, s0, s1)
}
