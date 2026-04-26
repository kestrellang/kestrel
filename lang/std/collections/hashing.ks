// Default hashing implementation using FNV-1a

module std.collections

import std.core.(Hasher, Hash, Defaultable)
import std.num.(UInt8, UInt64, Int64)
import std.memory.(Slice)

// ============================================================================
// DEFAULT HASHER
// ============================================================================

/// The standard `Hasher` implementation, backed by the 64-bit FNV-1a
/// algorithm.
///
/// Used by `Dictionary` and `Set` whenever the user doesn't pick a
/// specific hasher. FNV-1a (Fowler-Noll-Vo) is a non-cryptographic
/// hash with good distribution for short keys and trivial state (a
/// single 64-bit word). It is **not** suitable for adversarial
/// inputs — for HashDoS resistance, swap in a keyed hasher like
/// SipHash by spelling out `Dictionary[K, V, SipHasher]` directly.
/// Conforms to `Hasher` (the `write`/`finish` protocol used by
/// `Hash` implementations) and `Defaultable` (so generic code can
/// instantiate one with `H()`).
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
/// FNV-1a starts from the 64-bit offset basis
/// `0xcbf29ce484222325` and folds each byte by XOR-then-multiply
/// with the FNV prime `0x100000001b3`. The state stays a single
/// `UInt64` throughout, and `finish()` returns it directly.
///
/// # Representation
///
/// One `UInt64` field, `state`, holding the running FNV-1a digest.
public struct DefaultHasher: Hasher, Defaultable {
    /// Running 64-bit FNV-1a digest; updated by `write` and returned
    /// by `finish`.
    private var state: UInt64

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Empty
    /// Creates a fresh hasher initialized to the FNV-1a 64-bit offset
    /// basis.
    ///
    /// The starting `state` is `0xcbf29ce484222325` — the standard
    /// FNV-1a seed; the same input fed to two new hashers always
    /// produces the same `finish()` value, so this hasher is
    /// deterministic across runs (no random seeding).
    ///
    /// # Examples
    ///
    /// ```
    /// var h = DefaultHasher();
    /// h.finish();  // the offset basis itself, since nothing was written
    /// ```
    public init() {
        // FNV offset basis for 64-bit
        self.state = UInt64(intLiteral: 14695981039346656037);
    }

    // ========================================================================
    // HASHER PROTOCOL
    // ========================================================================

    /// Folds every byte of `bytes` into the running hash state.
    ///
    /// Implements the FNV-1a inner loop: `state = (state xor byte) *
    /// prime`. May be called any number of times before `finish()`;
    /// the result is identical to having received all the bytes in a
    /// single call. Safe to call with an empty slice (no-op).
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
        var i: Int64 = Int64(intLiteral: 0);
        let prime = UInt64(intLiteral: 1099511628211);

        while i < count {
            let byte = ptr.offset(by: i).read();
            // XOR state with byte
            self.state = self.state.bitwiseXor(UInt64(from: byte));
            // Multiply by prime
            self.state = self.state.multiply(prime);
            i = i + Int64(intLiteral: 1)
        }
    }

    /// Returns the current hash state as the final 64-bit digest.
    ///
    /// FNV-1a has no finalization step, so `finish()` simply reads
    /// `state` — calling it does not reset the hasher, so further
    /// `write()` calls would extend the same digest. Construct a
    /// fresh `DefaultHasher()` per logical hash to avoid accidental
    /// state reuse.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = DefaultHasher();
    /// h.write(bytes: "x".utf8Bytes());
    /// h.finish();  // 64-bit FNV-1a of "x"
    /// ```
    public mutating func finish() -> UInt64 {
        self.state
    }
}
