// Default hashing implementation: a streaming word-at-a-time mixer.

module std.collections

import std.core.(Hasher, Hashable, Defaultable)
import std.numeric.(UInt8, UInt64, Int64)
import std.collections.(Slice)

// ============================================================================
// DEFAULT HASHER
// ============================================================================

/// The standard `Hasher` implementation: a word-at-a-time
/// multiply-xorshift mixer with Murmur3's `fmix64` finalizer.
///
/// Used by `Dictionary` and `Set` whenever the user doesn't pick a
/// specific hasher. Input is folded in 8 bytes per round, and integers
/// (through `write(uint64:)`) take a single round. It streams: bytes are
/// buffered into whole words, so split writes hash the same as one write.
/// `finish()` folds in the total byte count and runs `fmix64`, so every
/// input bit avalanches across the output. Variable-length values end
/// their own input (`String` writes its length), which keeps `["ab", "c"]`
/// and `["a", "bc"]` apart.
///
/// **Not adversarially safe.** The mixer is unkeyed, so an attacker
/// who can choose keys can craft collisions. For HashDoS resistance,
/// swap in a keyed hasher by spelling out `Dictionary[K, V, MyHasher]`.
/// For non-adversarial workloads — internal IDs, parser symbols, config
/// values — this hasher is fast and distributes well.
///
/// # Examples
///
/// ```
/// var h = DefaultHasher();
/// "hello".hash(into: h);
/// let hash = h.finish();  // 64-bit hash of "hello"
///
/// // Used implicitly through the dictionary type alias:
/// let d: [String: Int64] = ["a": 1];   // DefaultHasher under the hood
/// ```
///
/// # Representation
///
/// `state` (the running digest), `pending` and `pendingCount` (up to 7
/// bytes not yet forming a word, little-endian), and `total` (bytes seen).
public struct DefaultHasher: Hasher, Defaultable {
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
