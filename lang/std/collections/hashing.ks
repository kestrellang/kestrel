// Default hashing implementation using a wyhash-derived per-byte mixer.

module std.collections

import std.core.(Hasher, Hash, Defaultable)
import std.num.(UInt8, UInt64, Int64)
import std.memory.(Slice)

// ============================================================================
// DEFAULT HASHER
// ============================================================================

/// The standard `Hasher` implementation, backed by a wyhash-derived
/// per-byte mixer.
///
/// Used by `Dictionary` and `Set` whenever the user doesn't pick a
/// specific hasher. Each byte folds into a 64-bit running state via
/// `state = (state ^ byte) * MULT`; `finish()` runs Murmur3's fmix64
/// finalizer to scramble the result so every input bit avalanches
/// across the output.
///
/// **Not adversarially safe.** The mixer is unkeyed, so an attacker
/// who can choose keys can craft collisions. For HashDoS resistance,
/// swap in a keyed hasher (planned: `SipHasher13`) by spelling out
/// `Dictionary[K, V, SipHasher13]` directly. For non-adversarial
/// workloads — internal IDs, parser symbols, config values — this
/// hasher is faster and has better distribution than FNV-1a.
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
/// # Algorithm
///
/// Initialization seeds `state` with the wyhash secret
/// `0x9e3779b97f4a7c15` (the "golden ratio" constant SplitMix64 uses).
/// Each byte updates the state with `state = (state ^ byte) *
/// 0x100000001b3`, which combines wyhash's mixing constant with
/// FNV-1a's prime so every bit of the byte propagates across the
/// 64-bit state. `finish()` runs Murmur3's `fmix64` finalizer
/// (xor-shift-multiply twice) so consecutive integer keys produce
/// non-clustered hashes.
///
/// # Representation
///
/// One `UInt64` field, `state`, holding the running digest.
public struct DefaultHasher: Hasher, Defaultable {
    /// Running 64-bit digest; updated by `write` and finalized by
    /// `finish`.
    private var state: UInt64

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Empty
    /// Creates a fresh hasher seeded with the SplitMix64 golden-ratio
    /// constant `0x9e3779b97f4a7c15`.
    ///
    /// The same input fed to two new hashers always produces the same
    /// `finish()` value — this hasher is deterministic across runs (no
    /// random seeding).
    public init() {
        // SplitMix64 / wyhash seed (golden ratio).
        self.state = UInt64(intLiteral: 11400714819323198485);
    }

    // ========================================================================
    // HASHER PROTOCOL
    // ========================================================================

    /// Folds every byte of `bytes` into the running hash state.
    ///
    /// May be called any number of times before `finish()`; the result
    /// is identical to having received all the bytes in a single call.
    /// Safe to call with an empty slice (no-op).
    ///
    /// # Examples
    ///
    /// ```
    /// var h = DefaultHasher();
    /// h.write(bytes: "hello".utf8Bytes());
    /// h.write(bytes: " world".utf8Bytes());
    /// // Equivalent to a single write of "hello world".utf8Bytes()
    /// ```
    public mutating func write(bytes: Slice[UInt8]) {
        let count = bytes.count;
        let ptr = bytes.pointer;
        // FNV prime, reused as the per-byte multiplier.
        let mult = UInt64(intLiteral: 1099511628211);
        var i: Int64 = Int64(intLiteral: 0);
        while i < count {
            let byte = ptr.offset(by: i).read();
            self.state = self.state.bitwiseXor(UInt64(from: byte));
            self.state = self.state.multiply(mult);
            // Per-byte avalanche: mix high bits down so adjacent
            // states diverge faster than plain FNV-1a.
            self.state = self.state.bitwiseXor(self.state.shiftRight(by: 32));
            i = i + Int64(intLiteral: 1)
        }
    }

    /// Returns the finalized 64-bit digest.
    ///
    /// Runs Murmur3's `fmix64` finalizer over the running state — two
    /// rounds of xor-shift-multiply that avalanche every input bit
    /// across the output. Consecutive integer keys (a common bucket
    /// query pattern) emerge well-distributed despite the simple
    /// mixer, which would otherwise leak the input's low-bit
    /// regularity into the bucket index.
    ///
    /// `finish()` mutates `state`; calling it twice on the same hasher
    /// is undefined — construct a fresh `DefaultHasher()` per logical
    /// hash.
    public mutating func finish() -> UInt64 {
        let m1 = UInt64(intLiteral: 18397679294719823053);  // 0xff51afd7ed558ccd
        let m2 = UInt64(intLiteral: 14181476777654086739);  // 0xc4ceb9fe1a85ec53
        var x = self.state;
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiply(m1);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiply(m2);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        self.state = x;
        x
    }
}
