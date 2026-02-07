// Float64 - 64-bit floating point (double precision)
// Generated from float.ks.template - DO NOT EDIT

// ============================================================================
// FLOAT64
// ============================================================================

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral, Convertible, Defaultable
)
import std.text.(String, Formattable, FormatOptions)
import std.num.(Int64, Float32)

/// A 64-bit IEEE 754 double-precision floating-point type.
///
/// Float64 supports arithmetic, comparison, mathematical functions, and
/// formatting. It can represent values from approximately ±1.8×10^308 with
/// about 15-17 significant decimal digits of precision.
///
/// Floating-point literals without a type annotation default to Float64:
///     let x = 3.14       // Float64
///     let y: Float32 = 3.14  // Float32 (explicit)
///
/// Special values:
/// - `Float64.nan`: Not-a-Number, the result of undefined operations like 0/0
/// - `Float64.infinity`: Positive infinity, the result of overflow or 1/0
/// - Negative zero (-0.0): Compares equal to positive zero
///
/// NaN behavior:
/// - NaN is not equal to anything, including itself: `nan == nan` is false
/// - NaN comparisons always return false: `nan < 0`, `nan > 0` are both false
/// - Any arithmetic with NaN produces NaN
///
/// Example:
///     let pi = Float64.pi
///     let area = pi * radius * radius
///     let formatted = area.format(options: .{precision: 2})  // "314.16"
public struct Float64:
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
    Convertible[Float32],
    FFISafe
{
    /// The underlying raw value.
    ///
    /// Direct access to the primitive `lang.f64` type. Useful for FFI
    /// or low-level operations.
    public var raw: lang.f64

    // ========================================================================
    // CONSTANTS - Basic Values
    // ========================================================================

    /// The zero value (0.0).
    ///
    /// Example:
    ///     Float64.zero  // 0.0
    public static var zero: Float64 { Float64(floatLiteral: 0.0) }

    /// The one value (1.0).
    ///
    /// Example:
    ///     Float64.one  // 1.0
    public static var one: Float64 { Float64(floatLiteral: 1.0) }

    /// The minimum finite value (-1.7976931348623157e308).
    ///
    /// The most negative representable finite number.
    ///
    /// Example:
    ///     Float64.minValue  // -1.7976931348623157e308
    public static var minValue: Float64 { Float64(floatLiteral: 1.7976931348623157e308).negate() }

    /// The maximum finite value (1.7976931348623157e308).
    ///
    /// The most positive representable finite number.
    ///
    /// Example:
    ///     Float64.maxValue  // 1.7976931348623157e308
    public static var maxValue: Float64 { Float64(floatLiteral: 1.7976931348623157e308) }

    /// The smallest positive normal value (2.2250738585072014e-308).
    ///
    /// Values smaller than this are subnormal (denormalized) and have
    /// reduced precision.
    ///
    /// Example:
    ///     Float64.minPositive  // 2.2250738585072014e-308
    public static var minPositive: Float64 { Float64(floatLiteral: 2.2250738585072014e-308) }

    /// Machine epsilon (2.220446049250313e-16).
    ///
    /// The smallest value such that `1.0 + epsilon != 1.0`.
    /// Useful for comparing floating-point values with tolerance.
    ///
    /// Example:
    ///     Float64.epsilon  // 2.220446049250313e-16
    ///
    ///     // Comparing with tolerance:
    ///     func almostEqual(a: Float64, b: Float64) -> Bool {
    ///         (a - b).abs() < Float64.epsilon * a.abs().max(b.abs())
    ///     }
    public static var epsilon: Float64 { Float64(floatLiteral: 2.220446049250313e-16) }

    // ========================================================================
    // CONSTANTS - Special Values
    // ========================================================================

    /// Positive infinity.
    ///
    /// The result of dividing a positive number by zero, or overflow.
    /// Negating infinity produces negative infinity.
    ///
    /// Example:
    ///     Float64.infinity       // inf
    ///     -Float64.infinity      // -inf
    ///     1.0 / 0.0              // inf
    ///     Float64.infinity + 1   // inf (still infinity)
    public static var infinity: Float64 { Float64(raw: lang.f64_infinity()) }

    /// Not-a-Number (NaN).
    ///
    /// The result of undefined mathematical operations like 0/0 or sqrt(-1).
    /// NaN has special comparison behavior: it is not equal to anything,
    /// including itself.
    ///
    /// Example:
    ///     Float64.nan           // nan
    ///     0.0 / 0.0             // nan
    ///     Float64.nan == Float64.nan  // false (!)
    ///     Float64.nan.isNaN     // true (use this to check for NaN)
    public static var nan: Float64 { Float64(raw: lang.f64_nan()) }

    // ========================================================================
    // CONSTANTS - Mathematical
    // ========================================================================

    /// The mathematical constant pi (3.14159265358979323846...).
    ///
    /// The ratio of a circle's circumference to its diameter.
    ///
    /// Example:
    ///     let circumference = 2.0 * Float64.pi * radius
    ///     let area = Float64.pi * radius * radius
    public static var pi: Float64 { Float64(floatLiteral: 3.141592653589793) }

    /// Euler's number e (2.71828182845904523536...).
    ///
    /// The base of the natural logarithm.
    ///
    /// Example:
    ///     let growth = (rate * time).exp()  // e^(rate*time)
    public static var e: Float64 { Float64(floatLiteral: 2.718281828459045) }

    /// Tau (6.28318530717958647692...).
    ///
    /// Equal to 2*pi. Some consider tau more natural for circular calculations
    /// since it represents one full turn.
    ///
    /// Example:
    ///     let circumference = Float64.tau * radius  // same as 2*pi*r
    public static var tau: Float64 { Float64(floatLiteral: 6.283185307179586) }

    /// The natural logarithm of 2 (0.693147180559945...).
    ///
    /// Example:
    ///     Float64.ln2  // 0.6931471805599453
    public static var ln2: Float64 { Float64(floatLiteral: 0.6931471805599453) }

    /// The natural logarithm of 10 (2.302585092994046...).
    ///
    /// Example:
    ///     Float64.ln10  // 2.302585092994046
    public static var ln10: Float64 { Float64(floatLiteral: 2.302585092994046) }

    /// The square root of 2 (1.4142135623730951...).
    ///
    /// Example:
    ///     Float64.sqrt2  // 1.4142135623730951
    public static var sqrt2: Float64 { Float64(floatLiteral: 1.4142135623730951) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates a Float64 from a floating-point literal.
    ///
    /// This initializer is called implicitly when using float literals.
    ///
    /// Example:
    ///     let x: Float64 = 3.14
    ///     let y = Float64(floatLiteral: 3.14)  // explicit, rarely needed
    public init(floatLiteral value: lang.f64) {
        self.raw = value
    }

    /// Creates a Float64 with the default value (zero).
    public init() {
        self.init(floatLiteral: 0.0)
    }

    /// Creates a Float64 from an integer literal.
    ///
    /// This allows using integer literals where Float64 is expected.
    /// The conversion is exact for integers up to 2^53.
    ///
    /// Example:
    ///     let x: Float64 = 42      // 42.0
    ///     let y = 3.14 + 1         // 4.14 (1 converted to Float64)
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f64(value)
    }

    /// Creates a Float64 from a raw `lang.f64` value.
    init(raw value: lang.f64) {
        self.raw = value
    }

    /// Creates a Float64 from an Int64.
    ///
    /// Note: Integers larger than 2^53 may lose precision.
    ///
    /// Example:
    ///     let n: Int64 = 42
    ///     let f = Float64(from: n)  // 42.0
    public init(from value: Int64) {
        self.raw = lang.cast_i64_f64(value.raw)
    }

    /// Creates a Float64 from a Float32.
    ///
    /// This conversion is always exact (widening).
    ///
    /// Example:
    ///     let f32: Float32 = 3.14
    ///     let f64 = Float64(from: f32)
    public init(from value: Float32) {
        self.raw = lang.cast_f32_f64(value.raw)
    }

    // ========================================================================
    // CLASSIFICATION (Properties)
    // ========================================================================

    /// Returns true if this value is NaN (Not-a-Number).
    ///
    /// This is the correct way to check for NaN. Do not use `x == Float64.nan`
    /// because NaN is not equal to itself.
    ///
    /// Example:
    ///     (0.0 / 0.0).isNaN     // true
    ///     Float64.nan.isNaN     // true
    ///     (1.0).isNaN           // false
    ///     Float64.infinity.isNaN  // false
    public var isNaN: Bool { get {
        Bool(boolLiteral: lang.f64_is_nan(self.raw))
    }}

    /// Returns true if this value is positive or negative infinity.
    ///
    /// Example:
    ///     Float64.infinity.isInfinite       // true
    ///     (-Float64.infinity).isInfinite    // true
    ///     (1.0 / 0.0).isInfinite            // true
    ///     (1.0).isInfinite                  // false
    ///     Float64.nan.isInfinite            // false
    public var isInfinite: Bool { get {
        Bool(boolLiteral: lang.f64_is_infinite(self.raw))
    }}

    /// Returns true if this value is finite (not NaN and not infinite).
    ///
    /// A value is finite if it's a normal number, subnormal number, or zero.
    ///
    /// Example:
    ///     (1.0).isFinite              // true
    ///     (0.0).isFinite              // true
    ///     Float64.infinity.isFinite   // false
    ///     Float64.nan.isFinite        // false
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
    ///     Float64.minPositive.isNormal  // true
    ///     (Float64.minPositive / 2.0).isNormal  // false (subnormal)
    public var isNormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() >= Float64.minPositive
    }}

    /// Returns true if this value is subnormal (denormalized).
    ///
    /// Subnormal numbers are very small numbers with reduced precision,
    /// between zero and minPositive.
    ///
    /// Example:
    ///     (1.0).isSubnormal           // false
    ///     (Float64.minPositive / 2.0).isSubnormal  // true
    ///     (0.0).isSubnormal           // false
    public var isSubnormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() < Float64.minPositive
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
    ///     Float64.nan.sign  // nan
    public var sign: Float64 { get {
        if self.isNaN { Float64.nan }
        else if self.isZero {
            let one = Float64.one;
            let inverse = one.divide(self);
            if inverse < 0.0 {
                let zero = Float64.zero;
                zero.negate()
            } else {
                Float64(floatLiteral: 0.0)
            }
        }
        else if self < 0.0 { Float64(raw: lang.f64_neg(1.0)) }
        else { Float64(floatLiteral: 1.0) }
    }}

    /// Returns true if this value is greater than zero.
    ///
    /// Returns false for positive zero, NaN, and negative numbers.
    ///
    /// Example:
    ///     (3.14).isPositive   // true
    ///     (0.0).isPositive    // false
    ///     (-3.14).isPositive  // false
    ///     Float64.infinity.isPositive  // true
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
    ///     (-Float64.infinity).isNegative  // true
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

    /// Compares two Float64 values for equality.
    ///
    /// WARNING: NaN is not equal to anything, including itself.
    /// Use `isNaN` to check for NaN values.
    ///
    /// Positive and negative zero are considered equal.
    ///
    /// Example:
    ///     (3.14).equals(other: 3.14)    // true
    ///     (0.0).equals(other: -0.0)     // true
    ///     Float64.nan.equals(other: Float64.nan)  // false (!)
    public func equals(other: Float64) -> Bool {
        Bool(boolLiteral: lang.f64_eq(self.raw, other.raw))
    }

    /// Compares two Float64 values and returns their ordering.
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
    ///     (1.0).compare(other: Float64.infinity)  // Ordering.less
    public func compare(other: Float64) -> Ordering {
        if Bool(boolLiteral: lang.f64_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.f64_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = Float64
    type Subtractable.Output = Float64
    type Multipliable.Output = Float64
    type Divisible.Output = Float64
    type Negatable.Output = Float64

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
    public func add(other: Float64) -> Float64 { Float64(raw: lang.f64_add(self.raw, other.raw)) }

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
    public func subtract(other: Float64) -> Float64 { Float64(raw: lang.f64_sub(self.raw, other.raw)) }

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
    public func multiply(other: Float64) -> Float64 { Float64(raw: lang.f64_mul(self.raw, other.raw)) }

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
    public func divide(other: Float64) -> Float64 { Float64(raw: lang.f64_div(self.raw, other.raw)) }
    /// Returns the negation of this floating-point number.
    ///
    /// Negating NaN produces NaN. Negating infinity produces negative infinity.
    /// Negating negative zero produces positive zero.
    ///
    /// Example:
    ///     (3.14).negate()   // -3.14
    ///     (-3.14).negate()  // 3.14
    ///     -3.14             // -3.14 (operator form)
    public func negate() -> Float64 { Float64(raw: lang.f64_neg(self.raw)) }

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
    ///     Float64.infinity.abs()  // infinity
    public func abs() -> Float64 {
        if Bool(boolLiteral: lang.f64_lt(self.raw, 0.0)) { self.negate() } else { self }
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
    public func floor() -> Float64 { Float64(raw: lang.f64_floor(self.raw)) }

    /// Returns the smallest integer greater than or equal to this value.
    ///
    /// Rounds toward positive infinity.
    ///
    /// Example:
    ///     (3.2).ceil()   // 4.0
    ///     (3.0).ceil()   // 3.0
    ///     (-3.7).ceil()  // -3.0 (toward positive infinity)
    ///     (-3.0).ceil()  // -3.0
    public func ceil() -> Float64 { Float64(raw: lang.f64_ceil(self.raw)) }

    /// Returns the nearest integer, rounding half away from zero.
    ///
    /// Values exactly halfway between two integers round away from zero.
    ///
    /// Example:
    ///     (3.4).round()   // 3.0
    ///     (3.5).round()   // 4.0 (halfway rounds up)
    ///     (3.6).round()   // 4.0
    ///     (-3.5).round()  // -4.0 (halfway rounds away from zero)
    public func round() -> Float64 { Float64(raw: lang.f64_round(self.raw)) }

    /// Returns the integer part, truncating toward zero.
    ///
    /// Equivalent to `floor()` for positive numbers and `ceil()` for negative.
    ///
    /// Example:
    ///     (3.7).trunc()   // 3.0
    ///     (-3.7).trunc()  // -3.0 (toward zero, not -4.0)
    ///     (3.0).trunc()   // 3.0
    public func trunc() -> Float64 { Float64(raw: lang.f64_trunc(self.raw)) }

    /// Returns the fractional part (self - self.trunc()).
    ///
    /// The result has the same sign as self.
    ///
    /// Example:
    ///     (3.7).fract()   // 0.7
    ///     (-3.7).fract()  // -0.7
    ///     (3.0).fract()   // 0.0
    public func fract() -> Float64 {
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
    ///     Float64.infinity.sqrt()  // infinity
    public func sqrt() -> Float64 { Float64(raw: lang.f64_sqrt(self.raw)) }

    /// Returns the cube root.
    ///
    /// Unlike sqrt, cbrt handles negative numbers.
    ///
    /// Example:
    ///     (8.0).cbrt()    // 2.0
    ///     (-8.0).cbrt()   // -2.0
    ///     (27.0).cbrt()   // 3.0
    public func cbrt() -> Float64 {
        Float64(raw: libm_cbrt(self.raw))
    }

    /// Returns the hypotenuse (sqrt(self² + other²)).
    ///
    /// Computed in a way that avoids overflow for large values.
    ///
    /// Example:
    ///     (3.0).hypot(other: 4.0)  // 5.0
    public func hypot(other: Float64) -> Float64 {
        Float64(raw: libm_hypot(self.raw, other.raw))
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
    ///     Float64.infinity.exp()  // infinity
    ///     (-Float64.infinity).exp()  // 0.0
    public func exp() -> Float64 { Float64(raw: libm_exp(self.raw)) }

    /// Returns 2 raised to this power (2^self).
    ///
    /// Example:
    ///     (0.0).exp2()   // 1.0
    ///     (3.0).exp2()   // 8.0
    ///     (0.5).exp2()   // 1.4142135623730951 (sqrt(2))
    public func exp2() -> Float64 { Float64(raw: libm_exp2(self.raw)) }

    /// Returns e^self - 1, computed accurately for small values.
    ///
    /// For small x, `exp(x) - 1` loses precision due to cancellation.
    /// This function provides accurate results.
    ///
    /// Example:
    ///     (0.0).expm1()     // 0.0
    ///     (1e-10).expm1()   // ~1e-10 (accurate)
    ///     (1.0).expm1()     // 1.718281828459045
    public func expm1() -> Float64 { Float64(raw: libm_expm1(self.raw)) }

    /// Returns the natural logarithm (base e).
    ///
    /// Returns NaN for negative numbers, -infinity for zero.
    ///
    /// Example:
    ///     (1.0).ln()      // 0.0
    ///     Float64.e.ln()  // 1.0
    ///     (10.0).ln()     // 2.302585092994046
    ///     (0.0).ln()      // -infinity
    ///     (-1.0).ln()     // NaN
    public func ln() -> Float64 { Float64(raw: libm_log(self.raw)) }

    /// Returns ln(1 + self), computed accurately for small values.
    ///
    /// For small x, `ln(1 + x)` loses precision. This function provides
    /// accurate results.
    ///
    /// Example:
    ///     (0.0).ln1p()     // 0.0
    ///     (1e-10).ln1p()   // ~1e-10 (accurate)
    ///     (1.0).ln1p()     // 0.6931471805599453 (ln(2))
    public func ln1p() -> Float64 { Float64(raw: libm_log1p(self.raw)) }

    /// Returns the base-2 logarithm.
    ///
    /// Example:
    ///     (1.0).log2()   // 0.0
    ///     (2.0).log2()   // 1.0
    ///     (8.0).log2()   // 3.0
    ///     (0.5).log2()   // -1.0
    public func log2() -> Float64 { Float64(raw: libm_log2(self.raw)) }

    /// Returns the base-10 logarithm.
    ///
    /// Example:
    ///     (1.0).log10()    // 0.0
    ///     (10.0).log10()   // 1.0
    ///     (100.0).log10()  // 2.0
    ///     (0.1).log10()    // -1.0
    public func log10() -> Float64 { Float64(raw: libm_log10(self.raw)) }

    /// Returns the logarithm with the given base.
    ///
    /// Equivalent to `self.ln() / base.ln()`.
    ///
    /// Example:
    ///     (8.0).log(base: 2.0)   // 3.0
    ///     (81.0).log(base: 3.0)  // 4.0
    public func log(base: Float64) -> Float64 {
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
    public func pow(exponent: Float64) -> Float64 {
        Float64(raw: libm_pow(self.raw, exponent.raw))
    }

    /// Returns self raised to an integer power.
    ///
    /// More efficient than `pow()` for integer exponents.
    ///
    /// Example:
    ///     (2.0).powi(exponent: 10)  // 1024.0
    ///     (2.0).powi(exponent: -1)  // 0.5
    public func powi(exponent: Int64) -> Float64 {
        if exponent == 0 { return Float64(floatLiteral: 1.0) };
        if exponent < 0 { return Float64(raw: lang.f64_div(1.0, self.powi(exponent.negate()).raw)) };
        var result = Float64(floatLiteral: 1.0);
        var base = self;
        var exp = exponent;
        while exp > 0 {
            if exp % 2 == 1 {
                result = Float64(raw: lang.f64_mul(result.raw, base.raw))
            };
            base = Float64(raw: lang.f64_mul(base.raw, base.raw));
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
    ///     (Float64.pi / 2).sin()  // 1.0
    ///     Float64.pi.sin()      // ~0.0 (small due to rounding)
    public func sin() -> Float64 { Float64(raw: libm_sin(self.raw)) }

    /// Returns the cosine (argument in radians).
    ///
    /// Example:
    ///     (0.0).cos()           // 1.0
    ///     (Float64.pi / 2).cos()  // ~0.0 (small due to rounding)
    ///     Float64.pi.cos()      // -1.0
    public func cos() -> Float64 { Float64(raw: libm_cos(self.raw)) }

    /// Returns the tangent (argument in radians).
    ///
    /// Example:
    ///     (0.0).tan()             // 0.0
    ///     (Float64.pi / 4).tan()  // 1.0
    public func tan() -> Float64 { Float64(raw: libm_tan(self.raw)) }

    /// Returns the arcsine (result in radians, range [-pi/2, pi/2]).
    ///
    /// Returns NaN if self is outside [-1, 1].
    ///
    /// Example:
    ///     (0.0).asin()   // 0.0
    ///     (1.0).asin()   // 1.5707963267948966 (pi/2)
    ///     (0.5).asin()   // 0.5235987755982989 (pi/6)
    ///     (2.0).asin()   // NaN
    public func asin() -> Float64 { Float64(raw: libm_asin(self.raw)) }

    /// Returns the arccosine (result in radians, range [0, pi]).
    ///
    /// Returns NaN if self is outside [-1, 1].
    ///
    /// Example:
    ///     (1.0).acos()   // 0.0
    ///     (0.0).acos()   // 1.5707963267948966 (pi/2)
    ///     (-1.0).acos()  // 3.141592653589793 (pi)
    ///     (2.0).acos()   // NaN
    public func acos() -> Float64 { Float64(raw: libm_acos(self.raw)) }

    /// Returns the arctangent (result in radians, range [-pi/2, pi/2]).
    ///
    /// Example:
    ///     (0.0).atan()   // 0.0
    ///     (1.0).atan()   // 0.7853981633974483 (pi/4)
    ///     Float64.infinity.atan()  // 1.5707963267948966 (pi/2)
    public func atan() -> Float64 { Float64(raw: libm_atan(self.raw)) }

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
    public func atan2(x: Float64) -> Float64 { Float64(raw: libm_atan2(self.raw, x.raw)) }

    /// Returns sine and cosine simultaneously.
    ///
    /// More efficient than calling sin() and cos() separately.
    ///
    /// Example:
    ///     let (s, c) = angle.sinCos()
    public func sinCos() -> (Float64, Float64) {
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
    public func sinh() -> Float64 { Float64(raw: libm_sinh(self.raw)) }

    /// Returns the hyperbolic cosine.
    ///
    /// Example:
    ///     (0.0).cosh()  // 1.0
    ///     (1.0).cosh()  // 1.5430806348152437
    public func cosh() -> Float64 { Float64(raw: libm_cosh(self.raw)) }

    /// Returns the hyperbolic tangent.
    ///
    /// Example:
    ///     (0.0).tanh()  // 0.0
    ///     (1.0).tanh()  // 0.7615941559557649
    ///     Float64.infinity.tanh()  // 1.0
    public func tanh() -> Float64 { Float64(raw: libm_tanh(self.raw)) }
    /// Returns the inverse hyperbolic sine.
    ///
    /// Example:
    ///     (0.0).asinh()  // 0.0
    ///     (1.0).asinh()  // 0.881373587019543
    public func asinh() -> Float64 { Float64(raw: libm_asinh(self.raw)) }

    /// Returns the inverse hyperbolic cosine.
    ///
    /// Returns NaN for values less than 1.
    ///
    /// Example:
    ///     (1.0).acosh()  // 0.0
    ///     (2.0).acosh()  // 1.3169578969248166
    ///     (0.5).acosh()  // NaN
    public func acosh() -> Float64 { Float64(raw: libm_acosh(self.raw)) }

    /// Returns the inverse hyperbolic tangent.
    ///
    /// Returns NaN for values outside (-1, 1).
    /// Returns ±infinity for ±1.
    ///
    /// Example:
    ///     (0.0).atanh()  // 0.0
    ///     (0.5).atanh()  // 0.5493061443340549
    ///     (1.0).atanh()  // infinity
    public func atanh() -> Float64 { Float64(raw: libm_atanh(self.raw)) }

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
    public func fma(a: Float64, b: Float64) -> Float64 {
        Float64(raw: lang.f64_fma(self.raw, a.raw, b.raw))
    }

    /// Returns a value with the magnitude of self and the sign of other.
    ///
    /// Example:
    ///     (3.14).copysign(from: -1.0)   // -3.14
    ///     (-3.14).copysign(from: 1.0)   // 3.14
    ///     Float64.nan.copysign(from: -1.0)  // -nan
    public func copysign(from other: Float64) -> Float64 {
        Float64(raw: lang.f64_copysign(self.raw, other.raw))
    }

    /// Returns the next representable value greater than self.
    ///
    /// For infinity, returns infinity. For the largest finite value,
    /// returns infinity.
    ///
    /// Example:
    ///     (0.0).nextUp()   // smallest positive subnormal
    ///     (1.0).nextUp()   // 1.0000000000000002
    public func nextUp() -> Float64 {
        Float64(raw: libm_nextafter(self.raw, lang.f64_infinity()))
    }

    /// Returns the next representable value less than self.
    ///
    /// For negative infinity, returns negative infinity.
    ///
    /// Example:
    ///     (0.0).nextDown()  // smallest negative subnormal (-0.0 first)
    ///     (1.0).nextDown()  // 0.9999999999999999
    public func nextDown() -> Float64 {
        Float64(raw: libm_nextafter(self.raw, lang.f64_neg(lang.f64_infinity())))
    }

    /// Returns the IEEE 754 remainder of self / other.
    ///
    /// Unlike the % operator (which uses truncated division), this uses
    /// round-to-nearest division as specified by IEEE 754.
    ///
    /// Example:
    ///     (5.0).remainder(dividingBy: 3.0)  // -1.0 (not 2.0)
    public func remainder(dividingBy other: Float64) -> Float64 {
        Float64(raw: libm_remainder(self.raw, other.raw))
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
    public func clamp(min: Float64, max: Float64) -> Float64 {
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
    public func lerp(to other: Float64, t: Float64) -> Float64 {
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
    ///     Float64.nan.toInt64()  // None
    ///     Float64.infinity.toInt64()  // None
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
        .Some(Int64(raw: lang.cast_f64_i64(truncated.raw)))
    }

    /// Converts to Float32.
    ///
    /// May lose precision or become infinity if the value is outside
    /// Float32's range.
    ///
    /// Example:
    ///     (3.14).toFloat32()  // 3.14 (approximately)
    public func toFloat32() -> Float32 {
        Float32(raw: lang.cast_f64_f32(self.raw))
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
    ///     Float64.parse(string: "3.14")      // Some(3.14)
    ///     Float64.parse(string: "-2.5e10")   // Some(-2.5e10)
    ///     Float64.parse(string: "inf")       // Some(infinity)
    ///     Float64.parse(string: "nan")       // Some(nan)
    ///     Float64.parse(string: "abc")       // None
    ///     Float64.parse(string: "")          // None
    public static func parse(string: String) -> Float64? {
        let len = string.byteCount;
        if len == 0 {
            return .None
        }

        // Check for special values
        // "nan"
        if len == 3 {
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            // 'n' or 'N' = 110 or 78
            // 'a' or 'A' = 97 or 65
            let isN0 = Int64(from: b0) == 110 or Int64(from: b0) == 78;
            let isA1 = Int64(from: b1) == 97 or Int64(from: b1) == 65;
            let isN2 = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            if isN0 and isA1 and isN2 {
                return .Some(Float64.nan)
            }
        }

        // "inf"
        if len == 3 {
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            // 'i' or 'I' = 105 or 73
            // 'n' or 'N' = 110 or 78
            // 'f' or 'F' = 102 or 70
            let isI = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            if isI and isN and isF {
                return .Some(Float64.infinity)
            }
        }

        // "-inf"
        if len == 4 {
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let isMinus = Int64(from: b0) == 45;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isMinus and isI and isN and isF {
                return .Some(Float64(raw: lang.f64_neg(lang.f64_infinity())))
            }
        }

        // "+inf"
        if len == 4 {
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let isPlus = Int64(from: b0) == 43;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isPlus and isI and isN and isF {
                return .Some(Float64.infinity)
            }
        }

        // "infinity"
        if len == 8 {
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
            if isI0 and isN1 and isF2 and isI3 and isN4 and isI5 and isT6 and isY7 {
                return .Some(Float64.infinity)
            }
        }

        // Parse regular number: [+-]?[0-9]*[.]?[0-9]*([eE][+-]?[0-9]+)?
        var index: Int64 = 0;
        var isNegative = false;

        // Check for sign
        let firstByte: UInt8 = string.byteAtUnchecked(0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {  // '-'
            isNegative = true;
            index = 1
        } else if firstByteVal == 43 {  // '+'
            index = 1
        }

        // Must have something after sign
        if index >= len {
            return .None
        }

        // Parse integer part - inline digit check (48='0', 57='9')
        var integerPart: Float64 = 0.0;
        var hasIntegerPart = false;
        var currentByte: Int64 = Int64(from: string.byteAtUnchecked(index));

        while index < len and currentByte >= 48 and currentByte <= 57 {
            let digit = Float64(from: currentByte - 48);
            integerPart = integerPart * 10.0 + digit;
            hasIntegerPart = true;
            index = index + 1;
            if index < len {
                currentByte = Int64(from: string.byteAtUnchecked(index))
            }
        }

        // Parse fractional part
        var fractionalPart: Float64 = 0.0;
        var hasFractionalPart = false;

        if index < len and currentByte == 46 {  // '.'
            index = index + 1;
            var divisor: Float64 = 10.0;

            if index < len {
                currentByte = Int64(from: string.byteAtUnchecked(index));
                while index < len and currentByte >= 48 and currentByte <= 57 {
                    let digit = Float64(from: currentByte - 48);
                    fractionalPart = fractionalPart + digit / divisor;
                    divisor = divisor * 10.0;
                    hasFractionalPart = true;
                    index = index + 1;
                    if index < len {
                        currentByte = Int64(from: string.byteAtUnchecked(index))
                    }
                }
            }
        }

        // Must have at least integer or fractional part
        if not hasIntegerPart and not hasFractionalPart {
            return .None
        }

        var result = integerPart + fractionalPart;

        // Parse exponent part
        if index < len and (currentByte == 101 or currentByte == 69) {  // 'e' or 'E'
            index = index + 1;

            if index >= len {
                return .None  // 'e' with no exponent
            }

            var expNegative = false;
            currentByte = Int64(from: string.byteAtUnchecked(index));

            if currentByte == 45 {  // '-'
                expNegative = true;
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }
            } else if currentByte == 43 {  // '+'
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }
            }

            if index >= len {
                return .None  // No exponent digits
            }

            var exponent: Int64 = 0;
            var hasExpDigit = false;

            while index < len and currentByte >= 48 and currentByte <= 57 {
                exponent = exponent * 10 + (currentByte - 48);
                hasExpDigit = true;
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }
            }

            if not hasExpDigit {
                return .None
            }

            // Apply exponent using pow
            let expFloat = Float64(from: exponent);
            let ten: Float64 = 10.0;
            if expNegative {
                result = result / ten.pow(expFloat)
            } else {
                result = result * ten.pow(expFloat)
            }
        }

        // Check for trailing characters
        if index != len {
            return .None
        }

        // Apply sign
        if isNegative {
            result = result.negate()
        }

        .Some(result)
    }

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
    /// - `alignment`: .Left, .Right, or .Center. Default: .Right
    /// - `sign`: .Negative (default), .Always, or .Space
    /// - `floatStyle`: .Fixed, .Scientific, .general, or .Percent
    ///
    /// Float styles:
    /// - `.Fixed`: Always use decimal notation (e.g., "3.14")
    /// - `.Scientific`: Always use exponential notation (e.g., "3.14e0")
    /// - `.general`: Choose notation based on magnitude (default)
    /// - `.Percent`: Multiply by 100 and add % (e.g., 0.5 -> "50%")
    ///
    /// Example:
    ///     (3.14159).format()  // "3.14159"
    ///
    ///     // Precision control
    ///     (3.14159).format(options: .{precision: 2})  // "3.14"
    ///     (3.14159).format(options: .{precision: 0})  // "3"
    ///
    ///     // Scientific notation
    ///     (1234.5).format(options: .{floatStyle: .Scientific})  // "1.2345e3"
    ///     (0.00123).format(options: .{floatStyle: .Scientific, precision: 2})  // "1.23e-3"
    ///
    ///     // Percentage
    ///     (0.756).format(options: .{floatStyle: .Percent})  // "75.6%"
    ///     (0.756).format(options: .{floatStyle: .Percent, precision: 0})  // "76%"
    ///
    ///     // Padding and alignment
    ///     (3.14).format(options: .{width: 8})  // "    3.14"
    ///     (3.14).format(options: .{width: 8, fill: '0'})  // "00003.14"
    ///     (3.14).format(options: .{width: 8, alignment: .Left})  // "3.14    "
    ///
    ///     // Sign display
    ///     (3.14).format(options: .{sign: .Always})  // "+3.14"
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
                let one = Float64.one;
                let inverse = one.divide(value);
                if inverse < 0.0 {
                    isNegative = true
                }
            }
            if isNegative {
                value = value.negate()
            }

            var style = options.floatStyle;
            if style == .Percent {
                value = value.multiply(100.0);
                suffixPercent = true;
                style = .Fixed
            }

            if style == .Auto {
                if precisionProvided == false {
                    trimTrailingZeros = true
                }
                if value.isZero {
                    style = .Fixed
                } else {
                    let expVal = value.log10().floor();
                    let expInt: Int64 = Int64(raw: lang.cast_f64_i64(expVal.raw));
                    if expInt < -4 or expInt >= precision {
                        style = .Scientific
                    } else {
                        style = .Fixed
                    }
                }
            }

            if style == .Scientific or style == .ScientificUpper {
                var exponent: Int64 = 0;
                var mantissa = value;
                if value.isZero == false {
                    let expVal = value.log10().floor();
                    exponent = Int64(raw: lang.cast_f64_i64(expVal.raw));
                    let pow10 = Float64(floatLiteral: 10.0).powi(exponent);
                    mantissa = value.divide(pow10);
                }

                let scale = Float64(floatLiteral: 10.0).powi(precision);
                mantissa = mantissa.multiply(scale).round().divide(scale);
                if mantissa >= 10.0 {
                    mantissa = mantissa.divide(10.0);
                    exponent = exponent + 1
                }

                let intPart = mantissa.trunc();
                var intVal: Int64 = Int64(raw: lang.cast_f64_i64(intPart.raw));

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
                    let ten: Float64 = 10.0;
                    while digitCount < precision {
                        fracPart = fracPart * ten;
                        let digit: Int64 = Int64(raw: lang.cast_f64_i64(fracPart.trunc().raw));
                        let charCode: Int64 = digit + 48;
                        number.appendByte(UInt8(from: charCode));
                        fracPart = fracPart - Float64(raw: lang.cast_i64_f64(digit.raw));
                        digitCount = digitCount + 1
                    }
                }

                if style == .ScientificUpper {
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
                    Float64(floatLiteral: 10.0).powi(precision)
                } else {
                    Float64(floatLiteral: 1.0)
                };

                var rounded = value;
                if precision >= 0 {
                    rounded = rounded.multiply(scale).round().divide(scale)
                }

                let intPart = rounded.trunc();
                var intVal: Int64 = Int64(raw: lang.cast_f64_i64(intPart.raw));

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
                    let ten: Float64 = 10.0;
                    while digitCount < precision {
                        fracPart = fracPart * ten;
                        let digit: Int64 = Int64(raw: lang.cast_f64_i64(fracPart.trunc().raw));
                        let charCode: Int64 = digit + 48;
                        number.appendByte(UInt8(from: charCode));
                        fracPart = fracPart - Float64(raw: lang.cast_i64_f64(digit.raw));
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
            } else if options.sign == .Always {
                result.appendByte(43)  // '+'
            } else if options.sign == .Space {
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
                if options.alignment == .Left {
                    padRight = padding
                } else if options.alignment == .Right {
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


// ============================================================================
// TYPE ALIASES
// ============================================================================

/// Default floating-point type.
///
/// Float is an alias for Float64. This is the recommended floating-point
/// type for most use cases, offering good precision and performance.
///
/// Example:
///     let pi: Float = 3.14159
///     let area = pi * radius * radius
public type Float = Float64
