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
/// `ChaCha8` for a fast, high-quality stream (seeded or OS-keyed),
/// `DefaultRandom` for the shared process-wide one, `SystemRandom` to read
/// the OS for every value, and `Lcg64` for the cheapest reproducible stream.
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
        let bits = Int64(truncating: self.nextUInt64().shiftRight(by: 11));
        Float64(from: bits) * (1.0 / 9007199254740992.0)
    }

    /// Returns a uniformly distributed `Float32` in `[0, 1)`.
    ///
    /// Uses the top 24 bits of one `nextUInt64()` draw scaled by `2^-24`.
    public mutating func nextFloat32() -> Float32 {
        let bits = Int64(truncating: self.nextUInt64().shiftRight(by: 40));
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
/// Its low bits are weak (the helpers on `RandomNumberGenerator` sample the
/// high bits). Prefer `ChaCha8(seed:)` for a reproducible stream and
/// `DefaultRandom` for unpredictable values.
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
        self.state = self.state.multiplyWrapping(a).addWrapping(c);
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
/// reproduce a stream. One system call per value makes it slow; the
/// no-argument conveniences use `DefaultRandom` (a `ChaCha8` keyed once from
/// the OS) instead.
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
        let rc = unsafe libc_getentropy(Pointer(to: value).asRaw(), 8);
        if rc != 0 {
            fatalError("SystemRandom: getentropy failed");
        }
        value
    }
}

// ============================================================================
// CHACHA8
// ============================================================================

/// A fast, seedable generator built on the ChaCha stream cipher with 8
/// rounds (Bernstein, 2008). Each block of the keystream gives 8 `UInt64`s.
///
/// ChaCha8 is unpredictable without its key and passes every statistical
/// test suite, which is why it backs the no-argument conveniences
/// (`Int64.random(in:)`, `shuffle()`, ...) through `DefaultRandom`. It's
/// also a good seeded generator: `ChaCha8(seed:)` always produces the same
/// stream. For keys, tokens and other secrets, use a dedicated crypto
/// package rather than a general-purpose generator.
///
/// # Examples
///
/// ```
/// var rng = ChaCha8(seed: 42);       // reproducible
/// let a = rng.nextUInt64();
/// var fresh = ChaCha8();             // keyed from OS entropy
/// ```
///
/// # Representation
///
/// The 256-bit key (`k0`…`k7`), a 64-bit block counter, and the current
/// block's 8 outputs (`b0`…`b7`) with the index of the next one. No heap
/// storage, so it's cheap to copy and to keep in a global.
public struct ChaCha8: RandomNumberGenerator, Defaultable {
    var k0: UInt32
    var k1: UInt32
    var k2: UInt32
    var k3: UInt32
    var k4: UInt32
    var k5: UInt32
    var k6: UInt32
    var k7: UInt32
    var counter: UInt64
    var b0: UInt64
    var b1: UInt64
    var b2: UInt64
    var b3: UInt64
    var b4: UInt64
    var b5: UInt64
    var b6: UInt64
    var b7: UInt64
    var index: Int64

    /// @name Default
    /// A generator keyed with 32 bytes of OS entropy (`getentropy`).
    public init() {
        var words: (UInt64, UInt64, UInt64, UInt64) = (0, 0, 0, 0);
        let rc = unsafe libc_getentropy(Pointer(to: words).asRaw(), 32);
        if rc != 0 {
            fatalError("ChaCha8: getentropy failed");
        }
        self.init(keyWords: words.0, words.1, words.2, words.3)
    }

    /// A reproducible generator: the same `seed` always gives the same
    /// stream. The seed is expanded to a 256-bit key with SplitMix64.
    ///
    /// # Examples
    ///
    /// ```
    /// var a = ChaCha8(seed: 7);
    /// var b = ChaCha8(seed: 7);
    /// assert(a.nextUInt64() == b.nextUInt64());
    /// ```
    public init(seed seed: UInt64) {
        var s = seed;
        let w0 = splitMix64(s);
        let w1 = splitMix64(s);
        let w2 = splitMix64(s);
        let w3 = splitMix64(s);
        self.init(keyWords: w0, w1, w2, w3)
    }

    /// A generator with an explicit 256-bit key, given as four
    /// little-endian 64-bit words. The stream starts at block 0.
    public init(keyWords w0: UInt64, w1: UInt64, w2: UInt64, w3: UInt64) {
        self.k0 = UInt32(truncating: w0);
        self.k1 = UInt32(truncating: w0.shiftRight(by: 32));
        self.k2 = UInt32(truncating: w1);
        self.k3 = UInt32(truncating: w1.shiftRight(by: 32));
        self.k4 = UInt32(truncating: w2);
        self.k5 = UInt32(truncating: w2.shiftRight(by: 32));
        self.k6 = UInt32(truncating: w3);
        self.k7 = UInt32(truncating: w3.shiftRight(by: 32));
        self.counter = 0;
        self.b0 = 0;
        self.b1 = 0;
        self.b2 = 0;
        self.b3 = 0;
        self.b4 = 0;
        self.b5 = 0;
        self.b6 = 0;
        self.b7 = 0;
        // Past the end, so the first call computes block 0.
        self.index = 8
    }

    /// The next 64 bits of the keystream. Computes a new block every 8
    /// calls.
    public mutating func nextUInt64() -> UInt64 {
        if self.index == 8 {
            self.refill();
            self.index = 0
        }
        let i = self.index;
        self.index = i + 1;
        match i {
            0 => self.b0,
            1 => self.b1,
            2 => self.b2,
            3 => self.b3,
            4 => self.b4,
            5 => self.b5,
            6 => self.b6,
            _ => self.b7
        }
    }

    /// Computes keystream block `counter` (8 rounds = 4 double rounds over
    /// the 4x4 word state: constants, key, 64-bit counter, zero nonce) into
    /// `b0`…`b7`, then advances the counter.
    mutating func refill() {
        let c0: UInt32 = 1634760805;  // "expa"
        let c1: UInt32 = 857760878;   // "nd 3"
        let c2: UInt32 = 2036477234;  // "2-by"
        let c3: UInt32 = 1797285236;  // "te k"
        let n0 = UInt32(truncating: self.counter);
        let n1 = UInt32(truncating: self.counter.shiftRight(by: 32));
        let zero: UInt32 = 0;
        var x0 = c0; var x1 = c1; var x2 = c2; var x3 = c3;
        var x4 = self.k0; var x5 = self.k1; var x6 = self.k2; var x7 = self.k3;
        var x8 = self.k4; var x9 = self.k5; var x10 = self.k6; var x11 = self.k7;
        var x12 = n0; var x13 = n1; var x14 = zero; var x15 = zero;
        var round: Int64 = 0;
        while round < 4 {
            x0 = x0.addWrapping(x4); x12 = x12.bitwiseXor(x0).rotateLeft(by: 16);
            x8 = x8.addWrapping(x12); x4 = x4.bitwiseXor(x8).rotateLeft(by: 12);
            x0 = x0.addWrapping(x4); x12 = x12.bitwiseXor(x0).rotateLeft(by: 8);
            x8 = x8.addWrapping(x12); x4 = x4.bitwiseXor(x8).rotateLeft(by: 7);
            x1 = x1.addWrapping(x5); x13 = x13.bitwiseXor(x1).rotateLeft(by: 16);
            x9 = x9.addWrapping(x13); x5 = x5.bitwiseXor(x9).rotateLeft(by: 12);
            x1 = x1.addWrapping(x5); x13 = x13.bitwiseXor(x1).rotateLeft(by: 8);
            x9 = x9.addWrapping(x13); x5 = x5.bitwiseXor(x9).rotateLeft(by: 7);
            x2 = x2.addWrapping(x6); x14 = x14.bitwiseXor(x2).rotateLeft(by: 16);
            x10 = x10.addWrapping(x14); x6 = x6.bitwiseXor(x10).rotateLeft(by: 12);
            x2 = x2.addWrapping(x6); x14 = x14.bitwiseXor(x2).rotateLeft(by: 8);
            x10 = x10.addWrapping(x14); x6 = x6.bitwiseXor(x10).rotateLeft(by: 7);
            x3 = x3.addWrapping(x7); x15 = x15.bitwiseXor(x3).rotateLeft(by: 16);
            x11 = x11.addWrapping(x15); x7 = x7.bitwiseXor(x11).rotateLeft(by: 12);
            x3 = x3.addWrapping(x7); x15 = x15.bitwiseXor(x3).rotateLeft(by: 8);
            x11 = x11.addWrapping(x15); x7 = x7.bitwiseXor(x11).rotateLeft(by: 7);
            x0 = x0.addWrapping(x5); x15 = x15.bitwiseXor(x0).rotateLeft(by: 16);
            x10 = x10.addWrapping(x15); x5 = x5.bitwiseXor(x10).rotateLeft(by: 12);
            x0 = x0.addWrapping(x5); x15 = x15.bitwiseXor(x0).rotateLeft(by: 8);
            x10 = x10.addWrapping(x15); x5 = x5.bitwiseXor(x10).rotateLeft(by: 7);
            x1 = x1.addWrapping(x6); x12 = x12.bitwiseXor(x1).rotateLeft(by: 16);
            x11 = x11.addWrapping(x12); x6 = x6.bitwiseXor(x11).rotateLeft(by: 12);
            x1 = x1.addWrapping(x6); x12 = x12.bitwiseXor(x1).rotateLeft(by: 8);
            x11 = x11.addWrapping(x12); x6 = x6.bitwiseXor(x11).rotateLeft(by: 7);
            x2 = x2.addWrapping(x7); x13 = x13.bitwiseXor(x2).rotateLeft(by: 16);
            x8 = x8.addWrapping(x13); x7 = x7.bitwiseXor(x8).rotateLeft(by: 12);
            x2 = x2.addWrapping(x7); x13 = x13.bitwiseXor(x2).rotateLeft(by: 8);
            x8 = x8.addWrapping(x13); x7 = x7.bitwiseXor(x8).rotateLeft(by: 7);
            x3 = x3.addWrapping(x4); x14 = x14.bitwiseXor(x3).rotateLeft(by: 16);
            x9 = x9.addWrapping(x14); x4 = x4.bitwiseXor(x9).rotateLeft(by: 12);
            x3 = x3.addWrapping(x4); x14 = x14.bitwiseXor(x3).rotateLeft(by: 8);
            x9 = x9.addWrapping(x14); x4 = x4.bitwiseXor(x9).rotateLeft(by: 7);
            round = round + 1
        }
        self.b0 = joinWords(x0.addWrapping(c0), x1.addWrapping(c1));
        self.b1 = joinWords(x2.addWrapping(c2), x3.addWrapping(c3));
        self.b2 = joinWords(x4.addWrapping(self.k0), x5.addWrapping(self.k1));
        self.b3 = joinWords(x6.addWrapping(self.k2), x7.addWrapping(self.k3));
        self.b4 = joinWords(x8.addWrapping(self.k4), x9.addWrapping(self.k5));
        self.b5 = joinWords(x10.addWrapping(self.k6), x11.addWrapping(self.k7));
        self.b6 = joinWords(x12.addWrapping(n0), x13.addWrapping(n1));
        self.b7 = joinWords(x14.addWrapping(zero), x15.addWrapping(zero));
        self.counter = self.counter.addWrapping(1)
    }
}

/// `low | high << 32`: two keystream words as one little-endian output.
func joinWords(low: UInt32, high: UInt32) -> UInt64 {
    UInt64(from: low).bitwiseOr(UInt64(from: high).shiftLeft(by: 32))
}

/// SplitMix64 (Steele, Lea & Flood): advances `state` and returns a
/// well-mixed word. Used to expand a 64-bit seed into a key.
func splitMix64(mutating state: UInt64) -> UInt64 {
    state = state.addWrapping(11400714819323198485);  // 0x9e3779b97f4a7c15
    var z = state;
    z = z.bitwiseXor(z.shiftRight(by: 30)).multiplyWrapping(13787848793156543929);
    z = z.bitwiseXor(z.shiftRight(by: 27)).multiplyWrapping(10723151780598845931);
    z.bitwiseXor(z.shiftRight(by: 31))
}

// ============================================================================
// DEFAULT RANDOM
// ============================================================================

/// The process-wide generator behind `DefaultRandom`: one `ChaCha8`, keyed
/// from OS entropy on first use, so programs that never draw a random
/// number never pay for the `getentropy` call.
// No threads yet; once there are, this becomes per-thread state.
var defaultRandomState: ChaCha8 = ChaCha8(seed: 0);
var defaultRandomKeyed: Bool = false;

/// The generator behind all the no-argument conveniences
/// (`Int64.random(in:)`, `Bool.random()`, `shuffle()`, ...). A zero-sized
/// handle to one process-wide `ChaCha8` keyed from OS entropy on first use:
/// unpredictable, and one system call per process instead of one per
/// number. Pass a seeded `ChaCha8` or `Lcg64` to the `using:` overloads when
/// you need a reproducible stream.
///
/// # Examples
///
/// ```
/// var rng = DefaultRandom();
/// let value = rng.nextUInt64();
/// ```
public struct DefaultRandom: RandomNumberGenerator, Defaultable {
    /// @name Default
    /// A handle to the process-wide generator. Free to construct.
    public init() {}

    /// The next value from the process-wide `ChaCha8`.
    public mutating func nextUInt64() -> UInt64 {
        if not defaultRandomKeyed {
            defaultRandomState = ChaCha8();
            defaultRandomKeyed = true
        }
        defaultRandomState.nextUInt64()
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

    /// Returns `true` or `false` with equal probability, from `DefaultRandom`.
    ///
    /// # Examples
    ///
    /// ```
    /// if Bool.random() { }
    /// ```
    public static func random() -> Bool {
        var rng = DefaultRandom();
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

    /// Returns a uniform `Float64` in `[0, 1)`, from `DefaultRandom`.
    ///
    /// # Examples
    ///
    /// ```
    /// let p = Float64.random();
    /// ```
    public static func random() -> Float64 {
        var rng = DefaultRandom();
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

    /// Returns a uniform `Float64` in `[from, to)`, from `DefaultRandom`.
    ///
    /// # Examples
    ///
    /// ```
    /// let jitter = Float64.random(from: 0.0, to: 250.0);
    /// ```
    public static func random(from from: Float64, to to: Float64) -> Float64 {
        var rng = DefaultRandom();
        Float64.random(from: from, to: to, using: rng)
    }
}

extend Float32 {
    /// Returns a uniform `Float32` in `[0, 1)`, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Float32 {
        rng.nextFloat32()
    }

    /// Returns a uniform `Float32` in `[0, 1)`, from `DefaultRandom`.
    public static func random() -> Float32 {
        var rng = DefaultRandom();
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
        Int64(truncating: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int64` range, from `DefaultRandom`.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = Int64.random();
    /// ```
    public static func random() -> Int64 {
        var rng = DefaultRandom();
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
        let lo = UInt64(truncating: bounds.start);
        let span = UInt64(truncating: bounds.end).subtractWrapping(lo);
        if span == UInt64.maxValue {
            return Int64(truncating: rng.nextUInt64())
        }
        Int64(truncating: lo.addWrapping(rng.nextUInt64(below: span + 1)))
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    ///
    /// # Examples
    ///
    /// ```
    /// let roll = Int64.random(in: 1..=6);
    /// ```
    public static func random[R](in range: R) -> Int64 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
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
        var rng = DefaultRandom();
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
        let lo = UInt64(truncating: bounds.start);
        let span = UInt64(truncating: bounds.end) - lo;
        lo + rng.nextUInt64(below: span + 1)
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    public static func random[R](in range: R) -> UInt64 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
        UInt64.random(in: range, using: rng)
    }
}

extend Int8 {
    /// Returns a uniform value over the full `Int8` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Int8 {
        // Truncating conversion keeps the draw uniform: every Int8 value has
        // the same number of UInt64 preimages.
        Int8(truncating: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int8` range, from `DefaultRandom`.
    public static func random() -> Int8 {
        var rng = DefaultRandom();
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
        Int8(truncating: bounds.start + Int64(truncating: rng.nextUInt64(below: UInt64(truncating: span) + 1)))
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    public static func random[R](in range: R) -> Int8 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
        Int8.random(in: range, using: rng)
    }
}

extend Int16 {
    /// Returns a uniform value over the full `Int16` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Int16 {
        // Truncating conversion keeps the draw uniform: every Int16 value has
        // the same number of UInt64 preimages.
        Int16(truncating: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int16` range, from `DefaultRandom`.
    public static func random() -> Int16 {
        var rng = DefaultRandom();
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
        Int16(truncating: bounds.start + Int64(truncating: rng.nextUInt64(below: UInt64(truncating: span) + 1)))
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    public static func random[R](in range: R) -> Int16 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
        Int16.random(in: range, using: rng)
    }
}

extend Int32 {
    /// Returns a uniform value over the full `Int32` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> Int32 {
        // Truncating conversion keeps the draw uniform: every Int32 value has
        // the same number of UInt64 preimages.
        Int32(truncating: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `Int32` range, from `DefaultRandom`.
    public static func random() -> Int32 {
        var rng = DefaultRandom();
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
        Int32(truncating: bounds.start + Int64(truncating: rng.nextUInt64(below: UInt64(truncating: span) + 1)))
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    public static func random[R](in range: R) -> Int32 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
        Int32.random(in: range, using: rng)
    }
}

extend UInt8 {
    /// Returns a uniform value over the full `UInt8` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> UInt8 {
        // Truncating conversion keeps the draw uniform: every UInt8 value has
        // the same number of UInt64 preimages.
        UInt8(truncating: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `UInt8` range, from `DefaultRandom`.
    public static func random() -> UInt8 {
        var rng = DefaultRandom();
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
        UInt8(truncating: bounds.start + Int64(truncating: rng.nextUInt64(below: UInt64(truncating: span) + 1)))
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    public static func random[R](in range: R) -> UInt8 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
        UInt8.random(in: range, using: rng)
    }
}

extend UInt16 {
    /// Returns a uniform value over the full `UInt16` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> UInt16 {
        // Truncating conversion keeps the draw uniform: every UInt16 value has
        // the same number of UInt64 preimages.
        UInt16(truncating: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `UInt16` range, from `DefaultRandom`.
    public static func random() -> UInt16 {
        var rng = DefaultRandom();
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
        UInt16(truncating: bounds.start + Int64(truncating: rng.nextUInt64(below: UInt64(truncating: span) + 1)))
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    public static func random[R](in range: R) -> UInt16 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
        UInt16.random(in: range, using: rng)
    }
}

extend UInt32 {
    /// Returns a uniform value over the full `UInt32` range, drawn from `rng`.
    public static func random(mutating using rng: some RandomNumberGenerator) -> UInt32 {
        // Truncating conversion keeps the draw uniform: every UInt32 value has
        // the same number of UInt64 preimages.
        UInt32(truncating: rng.nextUInt64())
    }

    /// Returns a uniform value over the full `UInt32` range, from `DefaultRandom`.
    public static func random() -> UInt32 {
        var rng = DefaultRandom();
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
        UInt32(truncating: bounds.start + Int64(truncating: rng.nextUInt64(below: UInt64(truncating: span) + 1)))
    }

    /// Returns a uniform value in `range`, from `DefaultRandom`.
    public static func random[R](in range: R) -> UInt32 where R: RandomBounds[Int64] {
        var rng = DefaultRandom();
        UInt32.random(in: range, using: rng)
    }
}
