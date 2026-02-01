// Random number generation protocols and implementations

module std.num

import std.num.(UInt64, Int64)
import std.core.(Defaultable)

// ============================================================================
// RANDOM NUMBER GENERATOR PROTOCOL
// ============================================================================

/// Protocol for types that can generate random numbers.
///
/// Used by shuffling algorithms and other randomized operations.
/// Implementations should provide uniformly distributed values.
///
/// Example:
///     struct MyRng: RandomNumberGenerator {
///         var state: UInt64
///
///         mutating func nextUInt64() -> UInt64 {
///             // Update state and return value
///         }
///     }
public protocol RandomNumberGenerator {
    /// Returns a random 64-bit unsigned integer.
    ///
    /// Each call should return a new value from a uniform distribution
    /// over all possible UInt64 values.
    mutating func nextUInt64() -> UInt64
}

/// Extension providing convenience methods for RandomNumberGenerator.
extend RandomNumberGenerator {
    /// Returns a random integer in the range [0, upperBound).
    ///
    /// Uses modulo for simplicity. For very large bounds approaching UInt64.max,
    /// this may have slight bias.
    ///
    /// Example:
    ///     var rng = Lcg64()
    ///     let roll = rng.nextInt(below: 6)  // 0..5
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

/// A simple linear congruential generator (LCG) for random numbers.
///
/// Uses the constants from Numerical Recipes (period 2^64):
///   a = 6364136223846793005
///   c = 1442695040888963407
///
/// This is suitable for non-cryptographic uses like shuffling.
/// For cryptographic randomness, use a system RNG.
///
/// Example:
///     var rng = Lcg64(seed: 12345)
///     let value = rng.nextUInt64()
public struct Lcg64: RandomNumberGenerator, Defaultable {
    private var state: UInt64

    /// Creates an LCG with the given seed.
    ///
    /// Different seeds produce different sequences.
    ///
    /// Example:
    ///     var rng = Lcg64(seed: 42)
    public init(seed seed: UInt64) {
        self.state = seed;
    }

    /// Creates an LCG with a default seed.
    ///
    /// Note: This always produces the same sequence. For different
    /// sequences, provide different seeds via init(seed:).
    public init() {
        // Default seed
        self.state = UInt64(intLiteral: 88172645463325252);
    }

    /// Returns the next random value and advances the state.
    public mutating func nextUInt64() -> UInt64 {
        // LCG formula: state = state * a + c
        let a = UInt64(intLiteral: 6364136223846793005);
        let c = UInt64(intLiteral: 1442695040888963407);
        self.state = self.state.multiply(a).add(c);
        self.state
    }
}
