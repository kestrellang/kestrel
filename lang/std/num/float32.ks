// Float32 - 32-bit floating point (single precision)
// Generated from float.ks.template - DO NOT EDIT

// ============================================================================
// FLOAT32
// ============================================================================

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Formattable, FormatOptions,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral, Convertible, Defaultable
)
import std.text.(String)
import std.num.(Int64, Float64)

/// A 32-bit IEEE 754 single-precision floating-point type.
///
/// Float32 supports arithmetic, comparison, mathematical functions, and
/// formatting. It can represent values from approximately ±3.4×10^38 with
/// about 6-9 significant decimal digits of precision.
///
/// Floating-point literals without a type annotation default to Float64:
///     let x = 3.14       // Float64
///     let y: Float64 = 3.14  // Float64 (explicit)
///
/// Special values:
/// - `Float32.nan`: Not-a-Number, the result of undefined operations like 0/0
/// - `Float32.infinity`: Positive infinity, the result of overflow or 1/0
/// - Negative zero (-0.0): Compares equal to positive zero
///
/// NaN behavior:
/// - NaN is not equal to anything, including itself: `nan == nan` is false
/// - NaN comparisons always return false: `nan < 0`, `nan > 0` are both false
/// - Any arithmetic with NaN produces NaN
///
/// Example:
///     let pi = Float32.pi
///     let area = pi * radius * radius
///     let formatted = area.format(options: .{precision: 2})  // "314.16"
public struct Float32:
    Comparable,
    Equatable,
    Formattable,
    Addable,
    Subtractable,
    Multipliable,
    Divisible,
    Negatable,
    ExpressibleByFloatLiteral,
    ExpressibleByIntLiteral,
    Defaultable,
    Convertible[Int64],
    Convertible[Float64],
    FFISafe
{
    /// The underlying raw value.
    ///
    /// Direct access to the primitive `lang.f32` type. Useful for FFI
    /// or low-level operations.
    public var raw: lang.f32

    // ========================================================================
    // CONSTANTS - Basic Values
    // ========================================================================

    /// The zero value (0.0).
    ///
    /// Example:
    ///     Float32.zero  // 0.0
    public static var zero: Float32 { Float32(floatLiteral: 0.0) }

    /// The one value (1.0).
    ///
    /// Example:
    ///     Float32.one  // 1.0
    public static var one: Float32 { Float32(floatLiteral: 1.0) }

    /// The minimum finite value (-3.4028235e38).
    ///
    /// The most negative representable finite number.
    ///
    /// Example:
    ///     Float32.minValue  // -3.4028235e38
    public static var minValue: Float32 { Float32(floatLiteral: 3.4028235e38).negate() }

    /// The maximum finite value (3.4028235e38).
    ///
    /// The most positive representable finite number.
    ///
    /// Example:
    ///     Float32.maxValue  // 3.4028235e38
    public static var maxValue: Float32 { Float32(floatLiteral: 3.4028235e38) }

    /// The smallest positive normal value (1.17549435e-38).
    ///
    /// Values smaller than this are subnormal (denormalized) and have
    /// reduced precision.
    ///
    /// Example:
    ///     Float32.minPositive  // 1.17549435e-38
    public static var minPositive: Float32 { Float32(floatLiteral: 1.17549435e-38) }

    /// Machine epsilon (1.1920929e-7).
    ///
    /// The smallest value such that `1.0 + epsilon != 1.0`.
    /// Useful for comparing floating-point values with tolerance.
    ///
    /// Example:
    ///     Float32.epsilon  // 1.1920929e-7
    ///
    ///     // Comparing with tolerance:
    ///     func almostEqual(a: Float32, b: Float32) -> Bool {
    ///         (a - b).abs() < Float32.epsilon * a.abs().max(b.abs())
    ///     }
    public static var epsilon: Float32 { Float32(floatLiteral: 1.1920929e-7) }

    // ========================================================================
    // CONSTANTS - Special Values
    // ========================================================================

    /// Positive infinity.
    ///
    /// The result of dividing a positive number by zero, or overflow.
    /// Negating infinity produces negative infinity.
    ///
    /// Example:
    ///     Float32.infinity       // inf
    ///     -Float32.infinity      // -inf
    ///     1.0 / 0.0              // inf
    ///     Float32.infinity + 1   // inf (still infinity)
    public static var infinity: Float32 { Float32(raw: lang.f32_infinity()) }

    /// Not-a-Number (NaN).
    ///
    /// The result of undefined mathematical operations like 0/0 or sqrt(-1).
    /// NaN has special comparison behavior: it is not equal to anything,
    /// including itself.
    ///
    /// Example:
    ///     Float32.nan           // nan
    ///     0.0 / 0.0             // nan
    ///     Float32.nan == Float32.nan  // false (!)
    ///     Float32.nan.isNaN     // true (use this to check for NaN)
    public static var nan: Float32 { Float32(raw: lang.f32_nan()) }

    // ========================================================================
    // CONSTANTS - Mathematical
    // ========================================================================

    /// The mathematical constant pi (3.14159265358979323846...).
    ///
    /// The ratio of a circle's circumference to its diameter.
    ///
    /// Example:
    ///     let circumference = 2.0 * Float32.pi * radius
    ///     let area = Float32.pi * radius * radius
    public static var pi: Float32 { Float32(floatLiteral: 3.141592653589793) }

    /// Euler's number e (2.71828182845904523536...).
    ///
    /// The base of the natural logarithm.
    ///
    /// Example:
    ///     let growth = (rate * time).exp()  // e^(rate*time)
    public static var e: Float32 { Float32(floatLiteral: 2.718281828459045) }

    /// Tau (6.28318530717958647692...).
    ///
    /// Equal to 2*pi. Some consider tau more natural for circular calculations
    /// since it represents one full turn.
    ///
    /// Example:
    ///     let circumference = Float32.tau * radius  // same as 2*pi*r
    public static var tau: Float32 { Float32(floatLiteral: 6.283185307179586) }

    /// The natural logarithm of 2 (0.693147180559945...).
    ///
    /// Example:
    ///     Float32.ln2  // 0.6931471805599453
    public static var ln2: Float32 { Float32(floatLiteral: 0.6931471805599453) }

    /// The natural logarithm of 10 (2.302585092994046...).
    ///
    /// Example:
    ///     Float32.ln10  // 2.302585092994046
    public static var ln10: Float32 { Float32(floatLiteral: 2.302585092994046) }

    /// The square root of 2 (1.4142135623730951...).
    ///
    /// Example:
    ///     Float32.sqrt2  // 1.4142135623730951
    public static var sqrt2: Float32 { Float32(floatLiteral: 1.4142135623730951) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates a Float32 from a floating-point literal.
    ///
    /// This initializer is called implicitly when using float literals.
    ///
    /// Example:
    ///     let x: Float32 = 3.14
    ///     let y = Float32(floatLiteral: 3.14)  // explicit, rarely needed
    public init(floatLiteral value: lang.f64) {
        self.raw = lang.cast_f64_f32(value)
    }

    /// Creates a Float32 with the default value (zero).
    public init() {
        self.init(floatLiteral: 0.0)
    }

    /// Creates a Float32 from an integer literal.
    ///
    /// This allows using integer literals where Float32 is expected.
    /// The conversion is exact for integers up to 2^53.
    ///
    /// Example:
    ///     let x: Float32 = 42      // 42.0
    ///     let y = 3.14 + 1         // 4.14 (1 converted to Float32)
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f32(value)
    }

    /// Creates a Float32 from a raw `lang.f32` value.
    init(raw value: lang.f32) {
        self.raw = value
    }

    /// Creates a Float32 from an Int64.
    ///
    /// Note: Integers larger than 2^53 may lose precision.
    ///
    /// Example:
    ///     let n: Int64 = 42
    ///     let f = Float32(from: n)  // 42.0
    public init(from value: Int64) {
        self.raw = lang.cast_i64_f32(value.raw)
    }

    /// Creates a Float32 from a Float64.
    ///
    /// This conversion is always exact (widening).
    ///
    /// Example:
    ///     let f32: Float64 = 3.14
    ///     let f64 = Float32(from: f32)
    public init(from value: Float64) {
        self.raw = lang.cast_f64_f32(value.raw)
    }

    // ========================================================================
    // CLASSIFICATION (Properties)
    // ========================================================================

    /// Returns true if this value is NaN (Not-a-Number).
    ///
    /// This is the correct way to check for NaN. Do not use `x == Float32.nan`
    /// because NaN is not equal to itself.
    ///
    /// Example:
    ///     (0.0 / 0.0).isNaN     // true
    ///     Float32.nan.isNaN     // true
    ///     (1.0).isNaN           // false
    ///     Float32.infinity.isNaN  // false
    public var isNaN: Bool { get {
        Bool(boolLiteral: lang.f32_is_nan(self.raw))
    }}

    /// Returns true if this value is positive or negative infinity.
    ///
    /// Example:
    ///     Float32.infinity.isInfinite       // true
    ///     (-Float32.infinity).isInfinite    // true
    ///     (1.0 / 0.0).isInfinite            // true
    ///     (1.0).isInfinite                  // false
    ///     Float32.nan.isInfinite            // false
    public var isInfinite: Bool { get {
        Bool(boolLiteral: lang.f32_is_infinite(self.raw))
    }}

    /// Returns true if this value is finite (not NaN and not infinite).
    ///
    /// A value is finite if it's a normal number, subnormal number, or zero.
    ///
    /// Example:
    ///     (1.0).isFinite              // true
    ///     (0.0).isFinite              // true
    ///     Float32.infinity.isFinite   // false
    ///     Float32.nan.isFinite        // false
    public var isFinite: Bool { get {
        not self.isNaN and not self.isInfinite
    }}

    /// Returns true if this value is a normal number.
    ///
    /// A normal number has full precision. Zero, subnormal numbers,
    /// infinity, and NaN are not normal.
    ///
    /// Example:
    ///     (1.0).isNormal              // true
    ///     (0.0).isNormal              // false
    ///     Float32.minPositive.isNormal  // true
    ///     (Float32.minPositive / 2.0).isNormal  // false (subnormal)
    public var isNormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() >= Float32.minPositive
    }}

    /// Returns true if this value is subnormal (denormalized).
    ///
    /// Subnormal numbers are very small numbers with reduced precision,
    /// between zero and minPositive.
    ///
    /// Example:
    ///     (1.0).isSubnormal           // false
    ///     (Float32.minPositive / 2.0).isSubnormal  // true
    ///     (0.0).isSubnormal           // false
    public var isSubnormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() < Float32.minPositive
    }}

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Returns -1.0 if negative, 0.0 if zero, or 1.0 if positive.
    ///
    /// For NaN, returns NaN. Negative zero returns -0.0 for sign but
    /// compares equal to 0.0.
    ///
    /// Example:
    ///     (-3.14).sign  // -1.0
    ///     (0.0).sign    // 0.0
    ///     (3.14).sign   // 1.0
    ///     Float32.nan.sign  // nan
    public var sign: Float32 { get {
        if self.isNaN { Float32.nan }
        else if self.isZero {
            let inverse = Float32.one.divide(other: self);
            if inverse < 0.0 { Float32.zero.negate() } else { Float32(floatLiteral: 0.0) }
        }
        else if self < 0.0 { Float32(raw: lang.f32_neg(1.0)) }
        else { Float32(floatLiteral: 1.0) }
    }}

    /// Returns true if this value is greater than zero.
    ///
    /// Returns false for positive zero, NaN, and negative numbers.
    ///
    /// Example:
    ///     (3.14).isPositive   // true
    ///     (0.0).isPositive    // false
    ///     (-3.14).isPositive  // false
    ///     Float32.infinity.isPositive  // true
    public var isPositive: Bool { get {
        self > 0.0
    }}

    /// Returns true if this value is less than zero.
    ///
    /// Returns false for negative zero, NaN, and positive numbers.
    ///
    /// Example:
    ///     (-3.14).isNegative  // true
    ///     (0.0).isNegative    // false
    ///     (3.14).isNegative   // false
    ///     (-Float32.infinity).isNegative  // true
    public var isNegative: Bool { get {
        self < 0.0
    }}

    /// Returns true if this value is positive or negative zero.
    ///
    /// Example:
    ///     (0.0).isZero   // true
    ///     (-0.0).isZero  // true
    ///     (0.1).isZero   // false
    public var isZero: Bool { get {
        self == 0.0
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Compares two Float32 values for equality.
    ///
    /// WARNING: NaN is not equal to anything, including itself.
    /// Use `isNaN` to check for NaN values.
    ///
    /// Positive and negative zero are considered equal.
    ///
    /// Example:
    ///     (3.14).equals(other: 3.14)    // true
    ///     (0.0).equals(other: -0.0)     // true
    ///     Float32.nan.equals(other: Float32.nan)  // false (!)
    public func equals(other: Float32) -> Bool {
        Bool(boolLiteral: lang.f32_eq(self.raw, other.raw))
    }

    /// Compares two Float32 values and returns their ordering.
    ///
    /// Returns `Ordering.less` if self < other, `Ordering.equal` if self == other,
    /// or `Ordering.greater` if self > other.
    ///
    /// WARNING: Comparisons involving NaN have undefined ordering behavior.
    /// Check `isNaN` before comparing if NaN values are possible.
    ///
    /// Example:
    ///     (1.0).compare(other: 2.0)   // Ordering.less
    ///     (2.0).compare(other: 2.0)   // Ordering.equal
    ///     (3.0).compare(other: 2.0)   // Ordering.greater
    ///
    ///     // Infinity compares as expected
    ///     (1.0).compare(other: Float32.infinity)  // Ordering.less
    public func compare(other: Float32) -> Ordering {
        if Bool(boolLiteral: lang.f32_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.f32_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = Float32
    type Subtractable.Output = Float32
    type Multipliable.Output = Float32
    type Divisible.Output = Float32
    type Negatable.Output = Float32

    // ========================================================================
    // ARITHMETIC
    // ========================================================================

    /// Adds two floating-point numbers.
    ///
    /// Special cases:
    /// - Adding anything to NaN produces NaN
    /// - infinity + infinity = infinity
    /// - infinity + (-infinity) = NaN
    /// - infinity + finite = infinity
    ///
    /// Example:
    ///     (1.5).add(other: 2.5)   // 4.0
    ///     1.5 + 2.5               // 4.0 (operator form)
    public func add(other: Float32) -> Float32 { Float32(raw: lang.f32_add(self.raw, other.raw)) }

    /// Subtracts another floating-point number from this one.
    ///
    /// Special cases:
    /// - Subtracting anything from NaN produces NaN
    /// - infinity - infinity = NaN
    /// - infinity - finite = infinity
    ///
    /// Example:
    ///     (5.0).subtract(other: 3.0)  // 2.0
    ///     5.0 - 3.0                   // 2.0 (operator form)
    public func subtract(other: Float32) -> Float32 { Float32(raw: lang.f32_sub(self.raw, other.raw)) }

    /// Multiplies two floating-point numbers.
    ///
    /// Special cases:
    /// - Multiplying anything by NaN produces NaN
    /// - infinity * 0 = NaN
    /// - infinity * finite (non-zero) = infinity (with appropriate sign)
    ///
    /// Example:
    ///     (2.5).multiply(other: 4.0)  // 10.0
    ///     2.5 * 4.0                   // 10.0 (operator form)
    public func multiply(other: Float32) -> Float32 { Float32(raw: lang.f32_mul(self.raw, other.raw)) }

    /// Divides this floating-point number by another.
    ///
    /// Unlike integer division, floating-point division by zero does not panic.
    /// Instead, it produces infinity or NaN.
    ///
    /// Special cases:
    /// - x / 0 = infinity (if x > 0), -infinity (if x < 0), or NaN (if x == 0)
    /// - x / infinity = 0 (for finite x)
    /// - infinity / infinity = NaN
    /// - Dividing anything by NaN produces NaN
    ///
    /// Example:
    ///     (10.0).divide(other: 4.0)  // 2.5
    ///     10.0 / 4.0                 // 2.5 (operator form)
    ///     1.0 / 0.0                  // infinity
    ///     0.0 / 0.0                  // NaN
    public func divide(other: Float32) -> Float32 { Float32(raw: lang.f32_div(self.raw, other.raw)) }
    /// Returns the negation of this floating-point number.
    ///
    /// Negating NaN produces NaN. Negating infinity produces negative infinity.
    /// Negating negative zero produces positive zero.
    ///
    /// Example:
    ///     (3.14).negate()   // -3.14
    ///     (-3.14).negate()  // 3.14
    ///     -3.14             // -3.14 (operator form)
    public func negate() -> Float32 { Float32(raw: lang.f32_neg(self.raw)) }

    // ========================================================================
    // BASIC MATHEMATICAL FUNCTIONS
    // ========================================================================

    /// Returns the absolute value.
    ///
    /// For NaN, returns NaN. For negative zero, returns positive zero.
    ///
    /// Example:
    ///     (3.14).abs()   // 3.14
    ///     (-3.14).abs()  // 3.14
    ///     (-0.0).abs()   // 0.0
    ///     Float32.infinity.abs()  // infinity
    public func abs() -> Float32 {
        if Bool(boolLiteral: lang.f32_lt(self.raw, 0.0)) { self.negate() } else { self }
    }

    /// Returns the largest integer less than or equal to this value.
    ///
    /// Rounds toward negative infinity.
    ///
    /// Example:
    ///     (3.7).floor()   // 3.0
    ///     (3.0).floor()   // 3.0
    ///     (-3.2).floor()  // -4.0 (toward negative infinity)
    ///     (-3.0).floor()  // -3.0
    public func floor() -> Float32 { Float32(raw: lang.f32_floor(self.raw)) }

    /// Returns the smallest integer greater than or equal to this value.
    ///
    /// Rounds toward positive infinity.
    ///
    /// Example:
    ///     (3.2).ceil()   // 4.0
    ///     (3.0).ceil()   // 3.0
    ///     (-3.7).ceil()  // -3.0 (toward positive infinity)
    ///     (-3.0).ceil()  // -3.0
    public func ceil() -> Float32 { Float32(raw: lang.f32_ceil(self.raw)) }

    /// Returns the nearest integer, rounding half away from zero.
    ///
    /// Values exactly halfway between two integers round away from zero.
    ///
    /// Example:
    ///     (3.4).round()   // 3.0
    ///     (3.5).round()   // 4.0 (halfway rounds up)
    ///     (3.6).round()   // 4.0
    ///     (-3.5).round()  // -4.0 (halfway rounds away from zero)
    public func round() -> Float32 { Float32(raw: lang.f32_round(self.raw)) }

    /// Returns the integer part, truncating toward zero.
    ///
    /// Equivalent to `floor()` for positive numbers and `ceil()` for negative.
    ///
    /// Example:
    ///     (3.7).trunc()   // 3.0
    ///     (-3.7).trunc()  // -3.0 (toward zero, not -4.0)
    ///     (3.0).trunc()   // 3.0
    public func trunc() -> Float32 { Float32(raw: lang.f32_trunc(self.raw)) }

    /// Returns the fractional part (self - self.trunc()).
    ///
    /// The result has the same sign as self.
    ///
    /// Example:
    ///     (3.7).fract()   // 0.7
    ///     (-3.7).fract()  // -0.7
    ///     (3.0).fract()   // 0.0
    public func fract() -> Float32 {
        self.subtract(self.trunc())
    }

    /// Returns the square root.
    ///
    /// Returns NaN for negative numbers (except -0.0 which returns -0.0).
    /// Returns infinity for positive infinity.
    ///
    /// Example:
    ///     (4.0).sqrt()    // 2.0
    ///     (2.0).sqrt()    // 1.4142135623730951
    ///     (0.0).sqrt()    // 0.0
    ///     (-1.0).sqrt()   // NaN
    ///     Float32.infinity.sqrt()  // infinity
    public func sqrt() -> Float32 { Float32(raw: lang.f32_sqrt(self.raw)) }

    /// Returns the cube root.
    ///
    /// Unlike sqrt, cbrt handles negative numbers.
    ///
    /// Example:
    ///     (8.0).cbrt()    // 2.0
    ///     (-8.0).cbrt()   // -2.0
    ///     (27.0).cbrt()   // 3.0
    public func cbrt() -> Float32 {
        Float32(raw: libm_cbrtf(self.raw))
    }

    /// Returns the hypotenuse (sqrt(self² + other²)).
    ///
    /// Computed in a way that avoids overflow for large values.
    ///
    /// Example:
    ///     (3.0).hypot(other: 4.0)  // 5.0
    public func hypot(other: Float32) -> Float32 {
        Float32(raw: libm_hypotf(self.raw, other.raw))
    }

    // ========================================================================
    // EXPONENTIAL AND LOGARITHMIC FUNCTIONS
    // ========================================================================

    /// Returns e raised to this power (e^self).
    ///
    /// Example:
    ///     (0.0).exp()   // 1.0
    ///     (1.0).exp()   // 2.718281828459045 (e)
    ///     (2.0).exp()   // 7.38905609893065
    ///     Float32.infinity.exp()  // infinity
    ///     (-Float32.infinity).exp()  // 0.0
    public func exp() -> Float32 { Float32(raw: libm_expf(self.raw)) }

    /// Returns 2 raised to this power (2^self).
    ///
    /// Example:
    ///     (0.0).exp2()   // 1.0
    ///     (3.0).exp2()   // 8.0
    ///     (0.5).exp2()   // 1.4142135623730951 (sqrt(2))
    public func exp2() -> Float32 { Float32(raw: libm_exp2f(self.raw)) }

    /// Returns e^self - 1, computed accurately for small values.
    ///
    /// For small x, `exp(x) - 1` loses precision due to cancellation.
    /// This function provides accurate results.
    ///
    /// Example:
    ///     (0.0).expm1()     // 0.0
    ///     (1e-10).expm1()   // ~1e-10 (accurate)
    ///     (1.0).expm1()     // 1.718281828459045
    public func expm1() -> Float32 { Float32(raw: libm_expm1f(self.raw)) }

    /// Returns the natural logarithm (base e).
    ///
    /// Returns NaN for negative numbers, -infinity for zero.
    ///
    /// Example:
    ///     (1.0).ln()      // 0.0
    ///     Float32.e.ln()  // 1.0
    ///     (10.0).ln()     // 2.302585092994046
    ///     (0.0).ln()      // -infinity
    ///     (-1.0).ln()     // NaN
    public func ln() -> Float32 { Float32(raw: libm_logf(self.raw)) }

    /// Returns ln(1 + self), computed accurately for small values.
    ///
    /// For small x, `ln(1 + x)` loses precision. This function provides
    /// accurate results.
    ///
    /// Example:
    ///     (0.0).ln1p()     // 0.0
    ///     (1e-10).ln1p()   // ~1e-10 (accurate)
    ///     (1.0).ln1p()     // 0.6931471805599453 (ln(2))
    public func ln1p() -> Float32 { Float32(raw: libm_log1pf(self.raw)) }

    /// Returns the base-2 logarithm.
    ///
    /// Example:
    ///     (1.0).log2()   // 0.0
    ///     (2.0).log2()   // 1.0
    ///     (8.0).log2()   // 3.0
    ///     (0.5).log2()   // -1.0
    public func log2() -> Float32 { Float32(raw: libm_log2f(self.raw)) }

    /// Returns the base-10 logarithm.
    ///
    /// Example:
    ///     (1.0).log10()    // 0.0
    ///     (10.0).log10()   // 1.0
    ///     (100.0).log10()  // 2.0
    ///     (0.1).log10()    // -1.0
    public func log10() -> Float32 { Float32(raw: libm_log10f(self.raw)) }

    /// Returns the logarithm with the given base.
    ///
    /// Equivalent to `self.ln() / base.ln()`.
    ///
    /// Example:
    ///     (8.0).log(base: 2.0)   // 3.0
    ///     (81.0).log(base: 3.0)  // 4.0
    public func log(base: Float32) -> Float32 {
        self.ln().divide(base.ln())
    }

    /// Returns self raised to the given power.
    ///
    /// Example:
    ///     (2.0).pow(exponent: 10.0)  // 1024.0
    ///     (2.0).pow(exponent: 0.5)   // 1.4142135623730951 (sqrt(2))
    ///     (2.0).pow(exponent: -1.0)  // 0.5
    ///     (-2.0).pow(exponent: 3.0)  // -8.0
    ///     (-2.0).pow(exponent: 0.5)  // NaN (negative base, non-integer exponent)
    public func pow(exponent: Float32) -> Float32 {
        Float32(raw: libm_powf(self.raw, exponent.raw))
    }

    /// Returns self raised to an integer power.
    ///
    /// More efficient than `pow()` for integer exponents.
    ///
    /// Example:
    ///     (2.0).powi(exponent: 10)  // 1024.0
    ///     (2.0).powi(exponent: -1)  // 0.5
    public func powi(exponent: Int64) -> Float32 {
        if exponent == 0 { return Float32(floatLiteral: 1.0) };
        if exponent < 0 { return Float32(raw: lang.f32_div(1.0, self.powi(exponent.negate()).raw)) };
        var result = Float32(floatLiteral: 1.0);
        var base = self;
        var exp = exponent;
        while exp > 0 {
            if exp % 2 == 1 {
                result = Float32(raw: lang.f32_mul(result.raw, base.raw))
            };
            base = Float32(raw: lang.f32_mul(base.raw, base.raw));
            exp = exp / 2
        };
        result
    }

    // ========================================================================
    // TRIGONOMETRIC FUNCTIONS
    // ========================================================================

    /// Returns the sine (argument in radians).
    ///
    /// Example:
    ///     (0.0).sin()           // 0.0
    ///     (Float32.pi / 2).sin()  // 1.0
    ///     Float32.pi.sin()      // ~0.0 (small due to rounding)
    public func sin() -> Float32 { Float32(raw: libm_sinf(self.raw)) }

    /// Returns the cosine (argument in radians).
    ///
    /// Example:
    ///     (0.0).cos()           // 1.0
    ///     (Float32.pi / 2).cos()  // ~0.0 (small due to rounding)
    ///     Float32.pi.cos()      // -1.0
    public func cos() -> Float32 { Float32(raw: libm_cosf(self.raw)) }

    /// Returns the tangent (argument in radians).
    ///
    /// Example:
    ///     (0.0).tan()             // 0.0
    ///     (Float32.pi / 4).tan()  // 1.0
    public func tan() -> Float32 { Float32(raw: libm_tanf(self.raw)) }

    /// Returns the arcsine (result in radians, range [-pi/2, pi/2]).
    ///
    /// Returns NaN if self is outside [-1, 1].
    ///
    /// Example:
    ///     (0.0).asin()   // 0.0
    ///     (1.0).asin()   // 1.5707963267948966 (pi/2)
    ///     (0.5).asin()   // 0.5235987755982989 (pi/6)
    ///     (2.0).asin()   // NaN
    public func asin() -> Float32 { Float32(raw: libm_asinf(self.raw)) }

    /// Returns the arccosine (result in radians, range [0, pi]).
    ///
    /// Returns NaN if self is outside [-1, 1].
    ///
    /// Example:
    ///     (1.0).acos()   // 0.0
    ///     (0.0).acos()   // 1.5707963267948966 (pi/2)
    ///     (-1.0).acos()  // 3.141592653589793 (pi)
    ///     (2.0).acos()   // NaN
    public func acos() -> Float32 { Float32(raw: libm_acosf(self.raw)) }

    /// Returns the arctangent (result in radians, range [-pi/2, pi/2]).
    ///
    /// Example:
    ///     (0.0).atan()   // 0.0
    ///     (1.0).atan()   // 0.7853981633974483 (pi/4)
    ///     Float32.infinity.atan()  // 1.5707963267948966 (pi/2)
    public func atan() -> Float32 { Float32(raw: libm_atanf(self.raw)) }

    /// Returns the two-argument arctangent of self/x (result in radians).
    ///
    /// Returns the angle between the positive x-axis and the point (x, self),
    /// in the range [-pi, pi]. Correctly handles all quadrants.
    ///
    /// Example:
    ///     (1.0).atan2(x: 1.0)    // 0.7853981633974483 (pi/4, first quadrant)
    ///     (1.0).atan2(x: -1.0)   // 2.356194490192345 (3*pi/4, second quadrant)
    ///     (-1.0).atan2(x: -1.0)  // -2.356194490192345 (third quadrant)
    ///     (-1.0).atan2(x: 1.0)   // -0.7853981633974483 (fourth quadrant)
    public func atan2(x: Float32) -> Float32 { Float32(raw: libm_atan2f(self.raw, x.raw)) }

    /// Returns sine and cosine simultaneously.
    ///
    /// More efficient than calling sin() and cos() separately.
    ///
    /// Example:
    ///     let (s, c) = angle.sinCos()
    public func sinCos() -> (Float32, Float32) {
        (self.sin(), self.cos())
    }

    // ========================================================================
    // HYPERBOLIC FUNCTIONS
    // ========================================================================

    /// Returns the hyperbolic sine.
    ///
    /// Example:
    ///     (0.0).sinh()  // 0.0
    ///     (1.0).sinh()  // 1.1752011936438014
    public func sinh() -> Float32 { Float32(raw: libm_sinhf(self.raw)) }

    /// Returns the hyperbolic cosine.
    ///
    /// Example:
    ///     (0.0).cosh()  // 1.0
    ///     (1.0).cosh()  // 1.5430806348152437
    public func cosh() -> Float32 { Float32(raw: libm_coshf(self.raw)) }

    /// Returns the hyperbolic tangent.
    ///
    /// Example:
    ///     (0.0).tanh()  // 0.0
    ///     (1.0).tanh()  // 0.7615941559557649
    ///     Float32.infinity.tanh()  // 1.0
    public func tanh() -> Float32 { Float32(raw: libm_tanhf(self.raw)) }
    /// Returns the inverse hyperbolic sine.
    ///
    /// Example:
    ///     (0.0).asinh()  // 0.0
    ///     (1.0).asinh()  // 0.881373587019543
    public func asinh() -> Float32 { Float32(raw: libm_asinhf(self.raw)) }

    /// Returns the inverse hyperbolic cosine.
    ///
    /// Returns NaN for values less than 1.
    ///
    /// Example:
    ///     (1.0).acosh()  // 0.0
    ///     (2.0).acosh()  // 1.3169578969248166
    ///     (0.5).acosh()  // NaN
    public func acosh() -> Float32 { Float32(raw: libm_acoshf(self.raw)) }

    /// Returns the inverse hyperbolic tangent.
    ///
    /// Returns NaN for values outside (-1, 1).
    /// Returns ±infinity for ±1.
    ///
    /// Example:
    ///     (0.0).atanh()  // 0.0
    ///     (0.5).atanh()  // 0.5493061443340549
    ///     (1.0).atanh()  // infinity
    public func atanh() -> Float32 { Float32(raw: libm_atanhf(self.raw)) }

    // ========================================================================
    // IEEE 754 OPERATIONS
    // ========================================================================

    /// Returns the fused multiply-add: (self * a) + b.
    ///
    /// Computed with only one rounding, which is more accurate and often
    /// faster than separate multiply and add operations.
    ///
    /// Example:
    ///     (2.0).fma(a: 3.0, b: 4.0)  // 10.0 (2*3 + 4)
    public func fma(a: Float32, b: Float32) -> Float32 {
        Float32(raw: lang.f32_fma(self.raw, a.raw, b.raw))
    }

    /// Returns a value with the magnitude of self and the sign of other.
    ///
    /// Example:
    ///     (3.14).copysign(from: -1.0)   // -3.14
    ///     (-3.14).copysign(from: 1.0)   // 3.14
    ///     Float32.nan.copysign(from: -1.0)  // -nan
    public func copysign(from other: Float32) -> Float32 {
        Float32(raw: lang.f32_copysign(self.raw, other.raw))
    }

    /// Returns the next representable value greater than self.
    ///
    /// For infinity, returns infinity. For the largest finite value,
    /// returns infinity.
    ///
    /// Example:
    ///     (0.0).nextUp()   // smallest positive subnormal
    ///     (1.0).nextUp()   // 1.0000000000000002
    public func nextUp() -> Float32 {
        Float32(raw: libm_nextafterf(self.raw, lang.f32_infinity()))
    }

    /// Returns the next representable value less than self.
    ///
    /// For negative infinity, returns negative infinity.
    ///
    /// Example:
    ///     (0.0).nextDown()  // smallest negative subnormal (-0.0 first)
    ///     (1.0).nextDown()  // 0.9999999999999999
    public func nextDown() -> Float32 {
        Float32(raw: libm_nextafterf(self.raw, lang.f32_neg(lang.f32_infinity())))
    }

    /// Returns the IEEE 754 remainder of self / other.
    ///
    /// Unlike the % operator (which uses truncated division), this uses
    /// round-to-nearest division as specified by IEEE 754.
    ///
    /// Example:
    ///     (5.0).remainder(dividingBy: 3.0)  // -1.0 (not 2.0)
    public func remainder(dividingBy other: Float32) -> Float32 {
        Float32(raw: libm_remainderf(self.raw, other.raw))
    }

    // ========================================================================
    // CLAMPING AND INTERPOLATION
    // ========================================================================

    /// Returns this value clamped to the given range.
    ///
    /// If self < min, returns min. If self > max, returns max.
    /// Otherwise returns self unchanged.
    ///
    /// Returns NaN if self is NaN.
    ///
    /// Example:
    ///     (0.5).clamp(min: 0.0, max: 1.0)   // 0.5
    ///     (-0.5).clamp(min: 0.0, max: 1.0)  // 0.0
    ///     (1.5).clamp(min: 0.0, max: 1.0)   // 1.0
    public func clamp(min: Float32, max: Float32) -> Float32 {
        if self.isNaN { self }
        else if self < min { min }
        else if self > max { max }
        else { self }
    }

    /// Linearly interpolates between self and other.
    ///
    /// Returns `self + (other - self) * t`.
    /// When t=0 returns self, when t=1 returns other.
    ///
    /// Example:
    ///     (0.0).lerp(to: 10.0, t: 0.0)   // 0.0
    ///     (0.0).lerp(to: 10.0, t: 0.5)   // 5.0
    ///     (0.0).lerp(to: 10.0, t: 1.0)   // 10.0
    ///     (0.0).lerp(to: 10.0, t: 0.25)  // 2.5
    public func lerp(to other: Float32, t: Float32) -> Float32 {
        self.add(other.subtract(self).multiply(t))
    }

    // ========================================================================
    // CONVERSION
    // ========================================================================

    /// Converts to Int64, truncating toward zero.
    ///
    /// Returns None if the value is NaN, infinite, or outside the range
    /// of Int64.
    ///
    /// Example:
    ///     (3.7).toInt64()    // Some(3)
    ///     (-3.7).toInt64()   // Some(-3)
    ///     Float32.nan.toInt64()  // None
    ///     Float32.infinity.toInt64()  // None
    public func toInt64() -> Int64? {
        if self.isNaN or self.isInfinite {
            return .None
        };
        let truncated = self.trunc();
        if truncated >= 9223372036854775808.0 {
            return .None
        };
        if truncated < -9223372036854775808.0 {
            return .None
        };
        .Some(Int64(raw: lang.cast_f32_i64(truncated.raw)))
    }

    /// Converts to Float64.
    ///
    /// May lose precision or become infinity if the value is outside
    /// Float64's range.
    ///
    /// Example:
    ///     (3.14).toFloat64()  // 3.14 (approximately)
    public func toFloat64() -> Float64 {
        Float64(raw: lang.cast_f32_f64(self.raw))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    /// Parses a float from a string.
    ///
    /// Accepts:
    /// - Decimal notation: "3.14", "-0.5", "+2.0"
    /// - Scientific notation: "1.5e10", "2.5E-3"
    /// - Special values: "inf", "-inf", "nan" (case insensitive)
    ///
    /// Returns None if the string is not a valid float.
    ///
    /// Example:
    ///     Float32.parse(string: "3.14")      // Some(3.14)
    ///     Float32.parse(string: "-2.5e10")   // Some(-2.5e10)
    ///     Float32.parse(string: "inf")       // Some(infinity)
    ///     Float32.parse(string: "nan")       // Some(nan)
    ///     Float32.parse(string: "abc")       // None
    ///     Float32.parse(string: "")          // None
    public static func parse(string: String) -> Float32? {{
        let len = string.byteCount;
        if len == 0 {{
            return .None
        }}

        // Check for special values
        // "nan"
        if len == 3 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            // 'n' or 'N' = 110 or 78
            // 'a' or 'A' = 97 or 65
            let isN0 = Int64(from: b0) == 110 or Int64(from: b0) == 78;
            let isA1 = Int64(from: b1) == 97 or Int64(from: b1) == 65;
            let isN2 = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            if isN0 and isA1 and isN2 {{
                return .Some(Float32.nan)
            }}
        }}

        // "inf"
        if len == 3 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            // 'i' or 'I' = 105 or 73
            // 'n' or 'N' = 110 or 78
            // 'f' or 'F' = 102 or 70
            let isI = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            if isI and isN and isF {{
                return .Some(Float32.infinity)
            }}
        }}

        // "-inf"
        if len == 4 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let isMinus = Int64(from: b0) == 45;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isMinus and isI and isN and isF {{
                return .Some(Float32(raw: lang.f32_neg(lang.f32_infinity())))
            }}
        }}

        // "+inf"
        if len == 4 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let isPlus = Int64(from: b0) == 43;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isPlus and isI and isN and isF {{
                return .Some(Float32.infinity)
            }}
        }}

        // "infinity"
        if len == 8 {{
            // Check for "infinity" (case insensitive)
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let b4: UInt8 = string.byteAtUnchecked(4);
            let b5: UInt8 = string.byteAtUnchecked(5);
            let b6: UInt8 = string.byteAtUnchecked(6);
            let b7: UInt8 = string.byteAtUnchecked(7);
            let isI0 = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN1 = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF2 = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            let isI3 = Int64(from: b3) == 105 or Int64(from: b3) == 73;
            let isN4 = Int64(from: b4) == 110 or Int64(from: b4) == 78;
            let isI5 = Int64(from: b5) == 105 or Int64(from: b5) == 73;
            let isT6 = Int64(from: b6) == 116 or Int64(from: b6) == 84;
            let isY7 = Int64(from: b7) == 121 or Int64(from: b7) == 89;
            if isI0 and isN1 and isF2 and isI3 and isN4 and isI5 and isT6 and isY7 {{
                return .Some(Float32.infinity)
            }}
        }}

        // Parse regular number: [+-]?[0-9]*[.]?[0-9]*([eE][+-]?[0-9]+)?
        var index: Int64 = 0;
        var isNegative = false;

        // Check for sign
        let firstByte: UInt8 = string.byteAtUnchecked(0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {{  // '-'
            isNegative = true;
            index = 1
        }} else if firstByteVal == 43 {{  // '+'
            index = 1
        }}

        // Must have something after sign
        if index >= len {{
            return .None
        }}

        // Parse integer part - inline digit check (48='0', 57='9')
        var integerPart: Float32 = 0.0;
        var hasIntegerPart = false;
        var currentByte: Int64 = Int64(from: string.byteAtUnchecked(index));

        while index < len and currentByte >= 48 and currentByte <= 57 {{
            let digit = Float32(from: currentByte - 48);
            integerPart = integerPart * 10.0 + digit;
            hasIntegerPart = true;
            index = index + 1;
            if index < len {{
                currentByte = Int64(from: string.byteAtUnchecked(index))
            }}
        }}

        // Parse fractional part
        var fractionalPart: Float32 = 0.0;
        var hasFractionalPart = false;

        if index < len and currentByte == 46 {{  // '.'
            index = index + 1;
            var divisor: Float32 = 10.0;

            if index < len {{
                currentByte = Int64(from: string.byteAtUnchecked(index));
                while index < len and currentByte >= 48 and currentByte <= 57 {{
                    let digit = Float32(from: currentByte - 48);
                    fractionalPart = fractionalPart + digit / divisor;
                    divisor = divisor * 10.0;
                    hasFractionalPart = true;
                    index = index + 1;
                    if index < len {{
                        currentByte = Int64(from: string.byteAtUnchecked(index))
                    }}
                }}
            }}
        }}

        // Must have at least integer or fractional part
        if not hasIntegerPart and not hasFractionalPart {{
            return .None
        }}

        var result = integerPart + fractionalPart;

        // Parse exponent part
        if index < len and (currentByte == 101 or currentByte == 69) {{  // 'e' or 'E'
            index = index + 1;

            if index >= len {{
                return .None  // 'e' with no exponent
            }}

            var expNegative = false;
            currentByte = Int64(from: string.byteAtUnchecked(index));

            if currentByte == 45 {{  // '-'
                expNegative = true;
                index = index + 1;
                if index < len {{
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }}
            }} else if currentByte == 43 {{  // '+'
                index = index + 1;
                if index < len {{
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }}
            }}

            if index >= len {{
                return .None  // No exponent digits
            }}

            var exponent: Int64 = 0;
            var hasExpDigit = false;

            while index < len and currentByte >= 48 and currentByte <= 57 {{
                exponent = exponent * 10 + (currentByte - 48);
                hasExpDigit = true;
                index = index + 1;
                if index < len {{
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }}
            }}

            if not hasExpDigit {{
                return .None
            }}

            // Apply exponent using pow
            let expFloat = Float32(from: exponent);
            let ten: Float32 = 10.0;
            if expNegative {{
                result = result / ten.pow(expFloat)
            }} else {{
                result = result * ten.pow(expFloat)
            }}
        }}

        // Check for trailing characters
        if index != len {{
            return .None
        }}

        // Apply sign
        if isNegative {{
            result = result.negate()
        }}

        .Some(result)
    }}

    // ========================================================================
    // FORMATTING
    // ========================================================================

    /// Formats this float as a string.
    ///
    /// Supports various formatting options including precision, width,
    /// padding, alignment, sign display, and float style.
    ///
    /// Format options:
    /// - `precision`: Number of digits after decimal point. Default: 6
    /// - `width`: Minimum output width. Default: 0
    /// - `fill`: Padding character. Default: ' '
    /// - `alignment`: .left, .right, or .center. Default: .right
    /// - `sign`: .negative (default), .always, or .space
    /// - `floatStyle`: .fixed, .scientific, .general, or .percent
    ///
    /// Float styles:
    /// - `.fixed`: Always use decimal notation (e.g., "3.14")
    /// - `.scientific`: Always use exponential notation (e.g., "3.14e0")
    /// - `.general`: Choose notation based on magnitude (default)
    /// - `.percent`: Multiply by 100 and add % (e.g., 0.5 -> "50%")
    ///
    /// Example:
    ///     (3.14159).format()  // "3.14159"
    ///
    ///     // Precision control
    ///     (3.14159).format(options: .{precision: 2})  // "3.14"
    ///     (3.14159).format(options: .{precision: 0})  // "3"
    ///
    ///     // Scientific notation
    ///     (1234.5).format(options: .{floatStyle: .scientific})  // "1.2345e3"
    ///     (0.00123).format(options: .{floatStyle: .scientific, precision: 2})  // "1.23e-3"
    ///
    ///     // Percentage
    ///     (0.756).format(options: .{floatStyle: .percent})  // "75.6%"
    ///     (0.756).format(options: .{floatStyle: .percent, precision: 0})  // "76%"
    ///
    ///     // Padding and alignment
    ///     (3.14).format(options: .{width: 8})  // "    3.14"
    ///     (3.14).format(options: .{width: 8, fill: '0'})  // "00003.14"
    ///     (3.14).format(options: .{width: 8, alignment: .left})  // "3.14    "
    ///
    ///     // Sign display
    ///     (3.14).format(options: .{sign: .always})  // "+3.14"
    ///
    ///     // String interpolation
    ///     "\{value}"       // general format
    ///     "\{value:.2}"    // 2 decimal places
    ///     "\{value:.2e}"   // scientific with 2 decimal places
    ///     "\{value:%}"     // percentage
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        var precision: Int64 = 6;
        var precisionProvided = false;
        if let .Some(p) = options.precision {
            precisionProvided = true;
            if p < 0 {
                precision = 0
            } else {
                precision = p
            }
        }

        var number = String();
        var isNegative = false;
        var allowSign = true;
        var suffixPercent = false;
        var trimTrailingZeros = false;
        var value = self;

        if self.isNaN {
            number = "NaN";
            allowSign = false;
        } else if self.isInfinite {
            number = "Infinity";
            isNegative = self < 0.0;
        } else {
            isNegative = value < 0.0;
            if value.isZero {
                let inverse = Float32.one.divide(other: value);
                if inverse < 0.0 {
                    isNegative = true
                }
            }
            if isNegative {
                value = value.negate()
            }

            var style = options.floatStyle;
            if style == .percent {
                value = value.multiply(other: 100.0);
                suffixPercent = true;
                style = .fixed
            }

            if style == .auto {
                if precisionProvided == false {
                    trimTrailingZeros = true
                }
                if value.isZero {
                    style = .fixed
                } else {
                    let expVal = value.log10().floor();
                    let expInt: Int64 = Int64(raw: lang.cast_f32_i64(expVal.raw));
                    if expInt < -4 or expInt >= precision {
                        style = .scientific
                    } else {
                        style = .fixed
                    }
                }
            }

            if style == .scientific or style == .scientificUpper {
                var exponent: Int64 = 0;
                var mantissa = value;
                if value.isZero == false {
                    let expVal = value.log10().floor();
                    exponent = Int64(raw: lang.cast_f32_i64(expVal.raw));
                    let pow10 = Float32(floatLiteral: 10.0).powi(exponent);
                    mantissa = value.divide(other: pow10);
                }

                let scale = Float32(floatLiteral: 10.0).powi(precision);
                mantissa = mantissa.multiply(other: scale).round().divide(other: scale);
                if mantissa >= 10.0 {
                    mantissa = mantissa.divide(other: 10.0);
                    exponent = exponent + 1
                }

                let intPart = mantissa.trunc();
                var intVal: Int64 = Int64(raw: lang.cast_f32_i64(intPart.raw));

                if intVal == 0 {
                    number.appendByte(48)
                } else {
                    var digits = String();
                    while intVal > 0 {
                        let digit: Int64 = intVal % 10;
                        let charCode: Int64 = digit + 48;
                        digits.appendByte(UInt8(from: charCode));
                        intVal = intVal / 10
                    }
                    var i = digits.byteCount - 1;
                    while i >= 0 {
                        number.appendByte(digits.byteAtUnchecked(i));
                        i = i - 1
                    }
                }

                if precision > 0 {
                    number.appendByte(46);
                    var fracPart = mantissa - intPart;
                    var digitCount: Int64 = 0;
                    let ten: Float32 = 10.0;
                    while digitCount < precision {
                        fracPart = fracPart * ten;
                        let digit: Int64 = Int64(raw: lang.cast_f32_i64(fracPart.trunc().raw));
                        let charCode: Int64 = digit + 48;
                        number.appendByte(UInt8(from: charCode));
                        fracPart = fracPart - Float32(raw: lang.cast_i64_f32(digit.raw));
                        digitCount = digitCount + 1
                    }
                }

                if style == .scientificUpper {
                    number.appendByte(69)  // 'E'
                } else {
                    number.appendByte(101)  // 'e'
                }

                var expVal: Int64 = exponent;
                if expVal < 0 {
                    number.appendByte(45);  // '-'
                    expVal = expVal.negate()
                }
                if expVal == 0 {
                    number.appendByte(48)  // '0'
                } else {
                    var digits = String();
                    while expVal > 0 {
                        let digit: Int64 = expVal % 10;
                        let charCode: Int64 = digit + 48;
                        digits.appendByte(UInt8(from: charCode));
                        expVal = expVal / 10
                    }
                    var i = digits.byteCount - 1;
                    while i >= 0 {
                        number.appendByte(digits.byteAtUnchecked(i));
                        i = i - 1
                    }
                }
            } else {
                let scale = if precision > 0 {
                    Float32(floatLiteral: 10.0).powi(precision)
                } else {
                    Float32(floatLiteral: 1.0)
                };

                var rounded = value;
                if precision >= 0 {
                    rounded = rounded.multiply(other: scale).round().divide(other: scale)
                }

                let intPart = rounded.trunc();
                var intVal: Int64 = Int64(raw: lang.cast_f32_i64(intPart.raw));

                if intVal == 0 {
                    number.appendByte(48)
                } else {
                    var digits = String();
                    while intVal > 0 {
                        let digit: Int64 = intVal % 10;
                        let charCode: Int64 = digit + 48;
                        digits.appendByte(UInt8(from: charCode));
                        intVal = intVal / 10
                    }
                    var i = digits.byteCount - 1;
                    while i >= 0 {
                        number.appendByte(digits.byteAtUnchecked(i));
                        i = i - 1
                    }
                }

                if precision > 0 {
                    number.appendByte(46);
                    var fracPart = rounded - intPart;
                    var digitCount: Int64 = 0;
                    let ten: Float32 = 10.0;
                    while digitCount < precision {
                        fracPart = fracPart * ten;
                        let digit: Int64 = Int64(raw: lang.cast_f32_i64(fracPart.trunc().raw));
                        let charCode: Int64 = digit + 48;
                        number.appendByte(UInt8(from: charCode));
                        fracPart = fracPart - Float32(raw: lang.cast_i64_f32(digit.raw));
                        digitCount = digitCount + 1
                    }
                }
            }

            if suffixPercent and precisionProvided == false {
                trimTrailingZeros = true
            }
        }

        var result = String();
        if allowSign {
            if isNegative {
                result.appendByte(45)  // '-'
            } else if options.sign == .always {
                result.appendByte(43)  // '+'
            } else if options.sign == .space {
                result.appendByte(32)  // ' '
            }
        }
        if trimTrailingZeros {
            let len = number.byteCount;
            var dotIndex: Int64 = -1;
            var expIndex: Int64 = -1;
            var i: Int64 = 0;
            while i < len {
                let b = number.byteAtUnchecked(i);
                let v = Int64(from: b);
                if v == 46 {  // '.'
                    dotIndex = i
                } else if v == 101 or v == 69 {  // 'e' or 'E'
                    expIndex = i;
                    break
                }
                i = i + 1
            }

            if dotIndex >= 0 {
                let endIndex: Int64 = if expIndex >= 0 { expIndex } else { len };
                var trimEnd = endIndex;
                while trimEnd > dotIndex + 1 {
                    let b = number.byteAtUnchecked(trimEnd - 1);
                    if Int64(from: b) == 48 {
                        trimEnd = trimEnd - 1
                    } else {
                        break
                    }
                }
                if trimEnd == dotIndex + 1 {
                    trimEnd = dotIndex
                }
                if trimEnd != endIndex {
                    var trimmed = String();
                    if trimEnd > 0 {
                        trimmed.append(number.substringBytes(from: 0, to: trimEnd))
                    }
                    if expIndex >= 0 {
                        trimmed.append(number.substringBytes(from: expIndex, to: len))
                    }
                    number = trimmed
                }
            }
        }

        result.append(number);
        if suffixPercent {
            result.appendByte(37)  // '%'
        }

        if let .Some(width) = options.width {
            if width > result.byteCount {
                var padLeft: Int64 = 0;
                var padRight: Int64 = 0;
                let padding = width - result.byteCount;
                if options.alignment == .left {
                    padRight = padding
                } else if options.alignment == .right {
                    padLeft = padding
                } else {
                    padLeft = padding / 2;
                    padRight = padding - padLeft
                }

                var padded = String();
                while padLeft > 0 {
                    padded.appendChar(options.fill);
                    padLeft = padLeft - 1
                }
                padded.append(result);
                while padRight > 0 {
                    padded.appendChar(options.fill);
                    padRight = padRight - 1
                }
                return padded
            }
        }

        result
    }}

