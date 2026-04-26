// Random number generation protocols and implementations

module std.num

import std.num.(UInt64, Int64)
import std.core.(Defaultable)

// ============================================================================
// RANDOM NUMBER GENERATOR PROTOCOL
// ============================================================================

/// A source of pseudo-random `UInt64` values. Implementers expose a single
/// raw-uniform primitive; the extension on this protocol layers ergonomic
/// helpers on top.
///
/// Conformers are free to choose any algorithm they like — the protocol
/// makes no statement about cryptographic strength, period, or bias. Pick
/// `Lcg64` for cheap non-cryptographic randomness; bring your own type for
/// anything stronger.
///
/// # Examples
///
/// ```
/// struct MyRng: RandomNumberGenerator {
///     var state: UInt64;
///
///     mutating func nextUInt64() -> UInt64 {
///         // mix state, return a fresh value
///     }
/// }
/// ```
public protocol RandomNumberGenerator {
    /// Returns the next `UInt64` from the stream and advances internal
    /// state. Each call should be independent and uniformly distributed
    /// over the full `UInt64` range — implementers that can't promise
    /// uniformity (e.g. very small periods) should document the bias.
    mutating func nextUInt64() -> UInt64
}

/// Convenience helpers built on top of `nextUInt64`.
extend RandomNumberGenerator {
    /// Returns a uniformly distributed integer in `[0, upperBound)`.
    /// Returns `0` when `upperBound <= 0` rather than panicking.
    ///
    /// Uses naive modulo for simplicity — for `upperBound` close to
    /// `UInt64.maxValue` the result has slight bias toward smaller values.
    /// If you need exact uniformity, sample `nextUInt64()` and reject.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// let roll = rng.nextInt(below: 6);   // 0..5
    /// ```
    public mutating func nextInt(below upperBound: Int64) -> Int64 {
        if upperBound <= Int64(intLiteral: 0) {
            return Int64(intLiteral: 0)
        }
        let bound = UInt64(from: upperBound);
        let value = self.nextUInt64();
        Int64(from: value.modulo(bound))
    }
}

// ============================================================================
// LINEAR CONGRUENTIAL GENERATOR
// ============================================================================

/// A 64-bit linear congruential generator. Cheap, allocation-free, and
/// adequate for shuffling, fuzz seeds, and simulation noise — *not* for
/// cryptographic use, key generation, or anything an adversary observes.
///
/// Constants come from Numerical Recipes and give a full period of `2^64`:
///
/// - multiplier `a = 6364136223846793005`
/// - increment  `c = 1442695040888963407`
///
/// The state update is `state = state * a + c`, returning the new state.
///
/// # Examples
///
/// ```
/// var rng = Lcg64(seed: 12345);
/// let v1 = rng.nextUInt64();
/// let v2 = rng.nextUInt64();   // distinct from v1
/// ```
///
/// # Representation
///
/// One `UInt64` field — the mutable generator state.
public struct Lcg64: RandomNumberGenerator, Defaultable {
    private var state: UInt64

    /// @name Seeded
    /// Creates a generator initialised with `seed`. Different seeds produce
    /// independent streams; the same seed always reproduces the same stream
    /// (useful for deterministic tests).
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// ```
    public init(seed seed: UInt64) {
        self.state = seed;
    }

    /// @name Default
    /// Creates a generator with a hard-coded default seed
    /// (`88172645463325252`). Always produces the same stream — provide an
    /// explicit seed via `init(seed:)` when you need variation between runs.
    public init() {
        // Default seed
        self.state = UInt64(intLiteral: 88172645463325252);
    }

    /// Advances the state once and returns the new value. `O(1)` and
    /// allocation-free.
    public mutating func nextUInt64() -> UInt64 {
        // LCG formula: state = state * a + c
        let a = UInt64(intLiteral: 6364136223846793005);
        let c = UInt64(intLiteral: 1442695040888963407);
        self.state = self.state.multiply(a).add(c);
        self.state
    }
}
