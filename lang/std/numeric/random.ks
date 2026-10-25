// Random number generation protocols and implementations

module std.numeric

import std.numeric.(UInt8, UInt16, UInt32, UInt64, Int8, Int16, Int32, Int64)
import std.numeric.(Float32, Float64, Steppable)
import std.core.(Bool, Comparable, Defaultable, Range, ClosedRange, fatalError)
import std.memory.(RawPointer, Pointer)

// ============================================================================
// RANDOM NUMBER GENERATOR PROTOCOL
// ============================================================================

/// A source of pseudo-random `UInt64` values. Implementers expose a single
/// raw-uniform primitive; the extension on this protocol layers ergonomic
/// helpers on top.
///
/// Conformers are free to choose any algorithm they like — the protocol
/// makes no statement about cryptographic strength, period, or bias. Pick
/// `Lcg64` for cheap reproducible randomness, `SystemRandom` for OS entropy;
/// bring your own type for anything stronger.
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
    /// Returns a uniformly distributed integer in `[0, upperBound)` with no
    /// modulo bias, using bitmask rejection sampling. Returns `0` when
    /// `upperBound` is `0`.
    ///
    /// Rejection draws at most 2 samples on average, so this stays `O(1)`
    /// in expectation for every bound.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// let roll = rng.nextUInt64(below: 6);   // 0..5, unbiased
    /// ```
    public mutating func nextUInt64(below upperBound: UInt64) -> UInt64 {
        if upperBound == 0 {
            return 0
        }
        let max = upperBound - 1;
        if max == 0 {
            return 0
        }
        // Keep only the top bits needed to cover max (LCG-class generators
        // have weak low bits, so we sample from the high end); the kept bits
        // are uniform over [0, 2^k - 1], and rejecting values above max
        // leaves [0, max] uniform.
        let shift = max.leadingZeros;
        loop {
            let candidate = self.nextUInt64().shiftRight(by: shift);
            if candidate <= max {
                return candidate
            }
        }
    }

    /// Returns a uniformly distributed `Float64` in `[0, 1)`.
    ///
    /// Uses the top 53 bits of one `nextUInt64()` draw scaled by `2^-53`,
    /// so every representable multiple of `2^-53` in the interval is
    /// equally likely.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// let p = rng.nextFloat64();   // e.g. 0.7297...
    /// ```
    public mutating func nextFloat64() -> Float64 {
        // Top 53 bits fit a Float64 mantissa exactly; the Int64 view of the
        // shifted value is always non-negative.
        let bits = Int64(from: self.nextUInt64().shiftRight(by: 11));
        Float64(from: bits) * (1.0 / 9007199254740992.0)
    }

    /// Returns a uniformly distributed `Float32` in `[0, 1)`.
    ///
    /// Uses the top 24 bits of one `nextUInt64()` draw scaled by `2^-24`.
    public mutating func nextFloat32() -> Float32 {
        let bits = Int64(from: self.nextUInt64().shiftRight(by: 40));
        Float32(from: Float64(from: bits) * (1.0 / 16777216.0))
    }

    /// Returns `true` or `false` with equal probability.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// if rng.nextBool() { }
    /// ```
    public mutating func nextBool() -> Bool {
        self.nextUInt64().shiftRight(by: 63) == 1
    }
}

// ============================================================================
// LINEAR CONGRUENTIAL GENERATOR
// ============================================================================

/// A 64-bit linear congruential generator. Cheap, allocation-free, and
/// adequate for shuffling, fuzz seeds, and simulation noise — *not* for
/// cryptographic use, key generation, or anything an adversary observes.
/// Reach for `SystemRandom` when you want unpredictable values, and for
/// `Lcg64(seed:)` when you want a reproducible stream.
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
        self.state = 88172645463325252;
    }

    /// Advances the state once and returns the new value. `O(1)` and
    /// allocation-free.
    public mutating func nextUInt64() -> UInt64 {
        // LCG formula: state = state * a + c
        let a = 6364136223846793005;
        let c = 1442695040888963407;
        self.state = self.state.multiply(a).add(c);
        self.state
    }
}

// ============================================================================
// SYSTEM RANDOM
// ============================================================================

/// `getentropy(2)` — fills `buf` with `len` OS entropy bytes (`len <= 256`).
/// Returns `0` on success. Available on macOS 10.12+ and glibc 2.25+.
@extern(.C, mangleName: "getentropy")
func libc_getentropy(consuming buf: RawPointer, consuming len: Int64) -> Int32

/// A random number generator backed by operating-system entropy.
///
/// Every `nextUInt64()` call reads 8 fresh bytes from the OS via
/// `getentropy(2)` — there is no seed, no internal state, and no way to
/// reproduce a stream. This is the generator behind all the no-argument
/// conveniences (`Int64.random(in:)`, `Bool.random()`, `shuffle()`, ...);
/// pass a seeded `Lcg64` to the `using:` overloads when you need
/// reproducibility instead.
///
/// Suitable as an entropy source, but see a dedicated crypto package for
/// key generation and other adversarial uses.
///
/// # Examples
///
/// ```
/// var rng = SystemRandom();
/// let value = rng.nextUInt64();   // unpredictable
/// ```
///
/// # Representation
///
/// Zero-sized — all state lives in the operating system.
public struct SystemRandom: RandomNumberGenerator, Defaultable {
    /// @name Default
    /// Creates a system generator. Construction is free; entropy is read
    /// per call.
    public init() {}

    /// Returns 8 fresh OS entropy bytes as a `UInt64`. Aborts the process
    /// in the (effectively impossible) case that `getentropy` fails.
    public mutating func nextUInt64() -> UInt64 {
        var value: UInt64 = 0;
        let rc = libc_getentropy(Pointer(to: value).asRaw(), 8);
        if rc != 0 {
            fatalError("SystemRandom: getentropy failed");
        }
        value
    }
}

// ============================================================================
// UNIFORM RANDOM VALUES — Bool
// ============================================================================

extend Bool {
    /// Returns `true` or `false` with equal probability, drawn from `rng`.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// let coin = Bool.random(using: rng);
    /// ```
    public static func random(mutating using rng: some RandomNumberGenerator) -> Bool {
        rng.nextBool()
    }

    /// Returns `true` or `false` with equal probability, using OS entropy.
    ///
    /// # Examples
    ///
    /// ```
    /// if Bool.random() { }
    /// ```
    public static func random() -> Bool {
        var rng = SystemRandom();
        Bool.random(using: rng)
    }
}

// ============================================================================
// UNIFORM RANDOM VALUES — floats
// ============================================================================

extend Float64 {
    /// Returns a uniform `Float64` in `[0, 1)`, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Float64 {
        rng.nextFloat64()
    }

    /// Returns a uniform `Float64` in `[0, 1)`, using OS entropy.
    ///
    /// # Examples
    ///
    /// ```
    /// let p = Float64.random();
    /// ```
    public static func random() -> Float64 {
        var rng = SystemRandom();
        Float64.random(using: rng)
    }

    /// Returns a uniform `Float64` in `[from, to)`, drawn from `rng`.
    /// Aborts when `from >= to`.
    public static func random(from from: Float64, to to: Float64, mutating using rng: some RandomNumberGenerator) -> Float64 {
        if from >= to {
            fatalError("Float64.random(from:to:): empty range");
        }
        from + (to - from) * rng.nextFloat64()
    }

    /// Returns a uniform `Float64` in `[from, to)`, using OS entropy.
    ///
    /// # Examples
    ///
    /// ```
    /// let jitter = Float64.random(from: 0.0, to: 250.0);
    /// ```
    public static func random(from from: Float64, to to: Float64) -> Float64 {
        var rng = SystemRandom();
        Float64.random(from: from, to: to, using: rng)
    }
}

extend Float32 {
    /// Returns a uniform `Float32` in `[0, 1)`, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Float32 {
        rng.nextFloat32()
    }

    /// Returns a uniform `Float32` in `[0, 1)`, using OS entropy.
    public static func random() -> Float32 {
        var rng = SystemRandom();
        Float32.random(using: rng)
    }
}

// ============================================================================
// RANDOM RANGES
// ============================================================================

/// Resolves a range-like type to inclusive bounds for uniform sampling —
/// the `random(in:)` analogue of `SeqRange`. `Range` and `ClosedRange`
/// conform for every standard integer type, so `Int64.random(in: 0..<6)`
/// and `Int64.random(in: 1..=6)` go through one generic entry point.
public protocol RandomBounds[B] {
    /// The inclusive `[start, end]` sampling bounds. Aborts the process
    /// when the range is empty (possible only for half-open ranges).
    func inclusiveBounds() -> ClosedRange[B]
}

extend Range[T]: RandomBounds[T] where T: Steppable, T: Comparable {
    public func inclusiveBounds() -> ClosedRange[T] {
        if self.start >= self.end {
            fatalError("random(in:): empty range");
        }
        ClosedRange[T](self.start, self.end.predecessor())
    }
}

extend ClosedRange[T]: RandomBounds[T] where T: Steppable, T: Comparable {
    public func inclusiveBounds() -> ClosedRange[T] { self }
}

// ============================================================================
// UNIFORM RANDOM VALUES — integers
// ============================================================================
//
// Every `random(in:)` takes its bounds in the Int64 domain (`RandomBounds[Int64]`)
// so integer-literal ranges — which default to Int64 — work at every target
// type: `Int8.random(in: -5..=5)`, `UInt32.random(in: 0..<100)`. Bounds are
// checked against the target type's representable range at runtime. For a
// uniform draw over a type's FULL range (including UInt64 values above
// Int64.maxValue), use the no-argument `random()` / `random(using:)` pair.
// All draws go through the unbiased `nextUInt64(below:)`.

extend Int64 {
    /// Returns a uniform value over the full `Int64` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Int64 {
        Int64(from: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int64` range, using OS entropy.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = Int64.random();
    /// ```
    public static func random() -> Int64 {
        var rng = SystemRandom();
        Int64.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Accepts both
    /// `..<` and `..=` ranges; aborts when the range is empty or inverted.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// let roll = Int64.random(in: 1..=6, using: rng);
    /// ```
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> Int64 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("Int64.random(in:): range start exceeds end");
        }
        // Work in the UInt64 bit domain: wrapping subtraction yields the
        // span even when it exceeds Int64.maxValue.
        let lo = UInt64(from: bounds.start);
        let span = UInt64(from: bounds.end).subtract(lo);
        if span == UInt64.maxValue {
            return Int64(from: rng.nextUInt64())
        }
        Int64(from: lo.add(rng.nextUInt64(below: span + 1)))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    ///
    /// # Examples
    ///
    /// ```
    /// let roll = Int64.random(in: 1..=6);
    /// ```
    public static func random[R](in range: R) -> Int64 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        Int64.random(in: range, using: rng)
    }
}

extend UInt64 {
    /// Returns a uniform value over the full `UInt64` range, drawn from
    /// `rng`. This is the only way to draw values above `Int64.maxValue` —
    /// `random(in:)` bounds live in the `Int64` domain.
    public static func random(mutating using rng: some RandomNumberGenerator) -> UInt64 {
        rng.nextUInt64()
    }

    /// Returns a uniform value over the full `UInt64` range, using OS
    /// entropy.
    public static func random() -> UInt64 {
        var rng = SystemRandom();
        UInt64.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Bounds are
    /// `Int64`-domain (so literal ranges work); negative bounds abort.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// let value = UInt64.random(in: 0..<100, using: rng);
    /// ```
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> UInt64 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("UInt64.random(in:): range start exceeds end");
        }
        if bounds.start < 0 {
            fatalError("UInt64.random(in:): negative bound");
        }
        let lo = UInt64(from: bounds.start);
        let span = UInt64(from: bounds.end) - lo;
        UInt64(from: lo + rng.nextUInt64(below: span + 1))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    public static func random[R](in range: R) -> UInt64 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        UInt64.random(in: range, using: rng)
    }
}

extend Int8 {
    /// Returns a uniform value over the full `Int8` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Int8 {
        // Truncating conversion keeps the draw uniform: every Int8 value has
        // the same number of UInt64 preimages.
        Int8(from: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int8` range, using OS entropy.
    public static func random() -> Int8 {
        var rng = SystemRandom();
        Int8.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Bounds are
    /// `Int64`-domain (so literal ranges work) and are checked against the
    /// `Int8` range at runtime; empty, inverted, or out-of-range bounds abort.
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> Int8 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("Int8.random(in:): range start exceeds end");
        }
        if bounds.start < Int64(from: Int8.minValue) or bounds.end > Int64(from: Int8.maxValue) {
            fatalError("Int8.random(in:): bounds exceed the Int8 range");
        }
        let span = bounds.end - bounds.start;
        Int8(from: bounds.start + Int64(from: rng.nextUInt64(below: UInt64(from: span) + 1)))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    public static func random[R](in range: R) -> Int8 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        Int8.random(in: range, using: rng)
    }
}

extend Int16 {
    /// Returns a uniform value over the full `Int16` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Int16 {
        // Truncating conversion keeps the draw uniform: every Int16 value has
        // the same number of UInt64 preimages.
        Int16(from: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int16` range, using OS entropy.
    public static func random() -> Int16 {
        var rng = SystemRandom();
        Int16.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Bounds are
    /// `Int64`-domain (so literal ranges work) and are checked against the
    /// `Int16` range at runtime; empty, inverted, or out-of-range bounds abort.
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> Int16 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("Int16.random(in:): range start exceeds end");
        }
        if bounds.start < Int64(from: Int16.minValue) or bounds.end > Int64(from: Int16.maxValue) {
            fatalError("Int16.random(in:): bounds exceed the Int16 range");
        }
        let span = bounds.end - bounds.start;
        Int16(from: bounds.start + Int64(from: rng.nextUInt64(below: UInt64(from: span) + 1)))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    public static func random[R](in range: R) -> Int16 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        Int16.random(in: range, using: rng)
    }
}

extend Int32 {
    /// Returns a uniform value over the full `Int32` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Int32 {
        // Truncating conversion keeps the draw uniform: every Int32 value has
        // the same number of UInt64 preimages.
        Int32(from: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int32` range, using OS entropy.
    public static func random() -> Int32 {
        var rng = SystemRandom();
        Int32.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Bounds are
    /// `Int64`-domain (so literal ranges work) and are checked against the
    /// `Int32` range at runtime; empty, inverted, or out-of-range bounds abort.
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> Int32 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("Int32.random(in:): range start exceeds end");
        }
        if bounds.start < Int64(from: Int32.minValue) or bounds.end > Int64(from: Int32.maxValue) {
            fatalError("Int32.random(in:): bounds exceed the Int32 range");
        }
        let span = bounds.end - bounds.start;
        Int32(from: bounds.start + Int64(from: rng.nextUInt64(below: UInt64(from: span) + 1)))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    public static func random[R](in range: R) -> Int32 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        Int32.random(in: range, using: rng)
    }
}

extend UInt8 {
    /// Returns a uniform value over the full `UInt8` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> UInt8 {
        // Truncating conversion keeps the draw uniform: every UInt8 value has
        // the same number of UInt64 preimages.
        UInt8(from: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `UInt8` range, using OS entropy.
    public static func random() -> UInt8 {
        var rng = SystemRandom();
        UInt8.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Bounds are
    /// `Int64`-domain (so literal ranges work) and are checked against the
    /// `UInt8` range at runtime; empty, inverted, or out-of-range bounds abort.
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> UInt8 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("UInt8.random(in:): range start exceeds end");
        }
        if bounds.start < 0 or bounds.end > Int64(from: UInt8.maxValue) {
            fatalError("UInt8.random(in:): bounds exceed the UInt8 range");
        }
        let span = bounds.end - bounds.start;
        UInt8(from: bounds.start + Int64(from: rng.nextUInt64(below: UInt64(from: span) + 1)))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    public static func random[R](in range: R) -> UInt8 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        UInt8.random(in: range, using: rng)
    }
}

extend UInt16 {
    /// Returns a uniform value over the full `UInt16` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> UInt16 {
        // Truncating conversion keeps the draw uniform: every UInt16 value has
        // the same number of UInt64 preimages.
        UInt16(from: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `UInt16` range, using OS entropy.
    public static func random() -> UInt16 {
        var rng = SystemRandom();
        UInt16.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Bounds are
    /// `Int64`-domain (so literal ranges work) and are checked against the
    /// `UInt16` range at runtime; empty, inverted, or out-of-range bounds abort.
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> UInt16 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("UInt16.random(in:): range start exceeds end");
        }
        if bounds.start < 0 or bounds.end > Int64(from: UInt16.maxValue) {
            fatalError("UInt16.random(in:): bounds exceed the UInt16 range");
        }
        let span = bounds.end - bounds.start;
        UInt16(from: bounds.start + Int64(from: rng.nextUInt64(below: UInt64(from: span) + 1)))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    public static func random[R](in range: R) -> UInt16 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        UInt16.random(in: range, using: rng)
    }
}

extend UInt32 {
    /// Returns a uniform value over the full `UInt32` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> UInt32 {
        // Truncating conversion keeps the draw uniform: every UInt32 value has
        // the same number of UInt64 preimages.
        UInt32(from: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `UInt32` range, using OS entropy.
    public static func random() -> UInt32 {
        var rng = SystemRandom();
        UInt32.random(using: rng)
    }

    /// Returns a uniform value in `range`, drawn from `rng`. Bounds are
    /// `Int64`-domain (so literal ranges work) and are checked against the
    /// `UInt32` range at runtime; empty, inverted, or out-of-range bounds abort.
    public static func random[R](in range: R, mutating using rng: some RandomNumberGenerator) -> UInt32 where R: RandomBounds[Int64] {
        let bounds = range.inclusiveBounds();
        if bounds.end < bounds.start {
            fatalError("UInt32.random(in:): range start exceeds end");
        }
        if bounds.start < 0 or bounds.end > Int64(from: UInt32.maxValue) {
            fatalError("UInt32.random(in:): bounds exceed the UInt32 range");
        }
        let span = bounds.end - bounds.start;
        UInt32(from: bounds.start + Int64(from: rng.nextUInt64(below: UInt64(from: span) + 1)))
    }

    /// Returns a uniform value in `range`, using OS entropy.
    public static func random[R](in range: R) -> UInt32 where R: RandomBounds[Int64] {
        var rng = SystemRandom();
        UInt32.random(in: range, using: rng)
    }
}
