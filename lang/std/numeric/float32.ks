// Float32 - 32-bit floating point (single precision)
// Generated from float.ks.template - DO NOT EDIT

// ============================================================================
// FLOAT32
// ============================================================================

module std.numeric

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral, Convertible, Defaultable
)
import std.text.(String, Formattable, FormatOptions)
import std.numeric.(Int64, Float64)

/// A 32-bit IEEE 754 single-precision float.
///
/// Range is approximately ±3.4×10^38 with 6-9 significant decimal
/// digits. Float literals without a type annotation default to `Float64`;
/// annotate the binding to pick `Float32`. The type is `FFISafe` and lays out
/// as a single `lang.f32`.
///
/// # Examples
///
/// ```
/// let pi = Float64.pi;
/// let area = pi * radius * radius;
/// let s = area.format(.{precision: 2});  // "314.16"
/// ```
///
/// ```
/// let x = 3.14;          // Float64
/// let y: Float32 = 3.14; // Float32
/// ```
///
/// # Special Values
///
/// - `nan` — Not-a-Number, result of `0.0 / 0.0`, `sqrt(-1)`, etc.
/// - `infinity` / `-infinity` — overflow or `1.0 / 0.0`.
/// - Negative zero compares equal to positive zero but produces `-infinity`
///   when used as a divisor.
///
/// NaN comparisons are surprising: `nan == nan` is false and every ordered
/// comparison against NaN is false. Use `isNaN` to test, never `== nan`. Any
/// arithmetic with NaN propagates NaN.
///
/// # Representation
///
/// A single `lang.f32` field holding the raw IEEE 754 bit pattern.
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
    /// The underlying primitive `lang.f32` value (IEEE 754 bit
    /// pattern). Exposed for FFI and intrinsic use; reach for the typed
    /// surface for everything else.
    public var raw: lang.f32

    // ========================================================================
    // CONSTANTS - Basic Values
    // ========================================================================

    /// The additive identity, `0.0`.
    public static var zero: Float32 { Float32(floatLiteral: 0.0) }

    /// The multiplicative identity, `1.0`.
    public static var one: Float32 { Float32(floatLiteral: 1.0) }

    /// The most negative finite value, ≈ -3.4028235e38.
    public static var minValue: Float32 { Float32(floatLiteral: 3.4028235e38).negate() }

    /// The most positive finite value, ≈ 3.4028235e38.
    public static var maxValue: Float32 { Float32(floatLiteral: 3.4028235e38) }

    /// The smallest positive *normal* value, ≈ 1.17549435e-38.
    /// Values smaller than this are subnormal and lose precision.
    public static var minPositive: Float32 { Float32(floatLiteral: 1.17549435e-38) }

    /// Machine epsilon — the smallest `e` such that `1.0 + e != 1.0`,
    /// ≈ 1.1920929e-7.
    ///
    /// Useful as a tolerance in approximate comparisons; scale by the
    /// operand magnitude for relative-error checks.
    ///
    /// # Examples
    ///
    /// ```
    /// func almostEqual(a: Float64, b: Float64) -> Bool {
    ///     (a - b).abs() < Float64.epsilon * a.abs().max(b.abs());
    /// }
    /// ```
    public static var epsilon: Float32 { Float32(floatLiteral: 1.1920929e-7) }

    // ========================================================================
    // CONSTANTS - Special Values
    // ========================================================================

    /// Positive infinity. Produced by overflow or `+x / 0.0` for `x > 0`.
    /// Arithmetic with infinity follows IEEE 754: finite + infinity is
    /// infinity, infinity − infinity is NaN.
    ///
    /// # Examples
    ///
    /// ```
    /// Float64.infinity;       // inf
    /// Float64.infinity + 1;   // inf
    /// 1.0 / 0.0;              // inf
    /// Float64.infinity.negate();  // -inf
    /// ```
    public static var infinity: Float32 { Float32(raw: lang.f32_infinity()) }

    /// Not-a-Number. Produced by undefined operations like `0.0 / 0.0` or
    /// `sqrt(-1.0)`. NaN propagates through arithmetic and is unequal to
    /// every value including itself — always test with `isNaN`.
    ///
    /// # Examples
    ///
    /// ```
    /// Float64.nan.isNaN;             // true
    /// Float64.nan == Float64.nan;    // false (!)
    /// 0.0 / 0.0;                     // nan
    /// ```
    public static var nan: Float32 { Float32(raw: lang.f32_nan()) }

    // ========================================================================
    // CONSTANTS - Mathematical
    // ========================================================================

    /// The constant π ≈ 3.14159265358979… — circle circumference over diameter.
    public static var pi: Float32 { Float32(floatLiteral: 3.141592653589793) }

    /// Euler's number `e` ≈ 2.71828182845904… — base of the natural logarithm.
    public static var e: Float32 { Float32(floatLiteral: 2.718281828459045) }

    /// Tau ≈ 6.28318530717958… — equal to `2π`, often more natural for
    /// "one full turn" rotational math.
    public static var tau: Float32 { Float32(floatLiteral: 6.283185307179586) }

    /// Natural logarithm of 2, ≈ 0.69314718055994…
    public static var ln2: Float32 { Float32(floatLiteral: 0.6931471805599453) }

    /// Natural logarithm of 10, ≈ 2.30258509299404…
    public static var ln10: Float32 { Float32(floatLiteral: 2.302585092994046) }

    /// Square root of 2, ≈ 1.41421356237309…
    public static var sqrt2: Float32 { Float32(floatLiteral: 1.4142135623730951) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// @name Float Literal
    /// Compiler-emitted bridge for floating-point literals via
    /// `ExpressibleByFloatLiteral`. Rarely called directly.
    ///
    /// # Examples
    ///
    /// ```
    /// let x: Float64 = 3.14;                  // implicit
    /// let y = Float64(floatLiteral: 3.14);    // explicit
    /// ```
    public init(floatLiteral value: lang.f64) {
        self.raw = lang.cast_f64_f32(value)
    }

    /// @name Default
    /// Creates the zero value, satisfying `Defaultable`.
    public init() {
        self.init(floatLiteral: 0.0)
    }

    /// @name Int Literal
    /// Bridge that lets bare integer literals appear where a float is
    /// expected. Conversion is exact up to ±2^53; larger magnitudes round.
    ///
    /// # Examples
    ///
    /// ```
    /// let x: Float64 = 42;     // 42.0
    /// let y = 3.14 + 1;        // 4.14 — `1` widened to Float64
    /// ```
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f32(value)
    }

    /// @name From Raw
    /// Wraps an existing `lang.f32` bit pattern. Internal; used
    /// by intrinsics.
    init(raw value: lang.f32) {
        self.raw = value
    }

    /// @name From Int
    /// Converts an `Int64` to a float. Values with magnitude greater than
    /// 2^53 lose low-order bits.
    ///
    /// # Examples
    ///
    /// ```
    /// let n: Int64 = 42;
    /// let f = Float64(from: n);  // 42.0
    /// ```
    public init(from value: Int64) {
        self.raw = lang.cast_i64_f32(value.raw)
    }

    /// @name From Float
    /// Converts between `Float32` and `Float64`. The 32→64 direction is
    /// exact; the 64→32 direction rounds and may overflow to ±infinity.
    ///
    /// # Examples
    ///
    /// ```
    /// let f32: Float32 = 3.14;
    /// let f64 = Float64(from: f32);
    /// ```
    public init(from value: Float64) {
        self.raw = lang.cast_f64_f32(value.raw)
    }

    // ========================================================================
    // CLASSIFICATION (Properties)
    // ========================================================================

    /// True if `self` is NaN. The only correct way to test for NaN — `==`
    /// returns false against NaN even when both operands are NaN.
    ///
    /// # Examples
    ///
    /// ```
    /// (0.0 / 0.0).isNaN;        // true
    /// Float64.nan.isNaN;        // true
    /// (1.0).isNaN;              // false
    /// Float64.infinity.isNaN;   // false
    /// ```
    public var isNaN: Bool { get {
        Bool(boolLiteral: lang.f32_is_nan(self.raw))
    }}

    /// True if `self` is `+infinity` or `-infinity`.
    ///
    /// # Examples
    ///
    /// ```
    /// Float64.infinity.isInfinite;             // true
    /// Float64.infinity.negate().isInfinite;    // true
    /// (1.0 / 0.0).isInfinite;                  // true
    /// Float64.nan.isInfinite;                  // false
    /// ```
    public var isInfinite: Bool { get {
        Bool(boolLiteral: lang.f32_is_infinite(self.raw))
    }}

    /// True if `self` is finite — equivalently, not NaN and not infinite.
    /// Includes zero and subnormals.
    public var isFinite: Bool { get {
        not self.isNaN and not self.isInfinite
    }}

    /// True if `self` is a *normal* number — finite, non-zero, and at least
    /// `minPositive` in magnitude. Subnormals, zero, infinity, and NaN are
    /// not normal.
    ///
    /// # Examples
    ///
    /// ```
    /// (1.0).isNormal;                              // true
    /// (0.0).isNormal;                              // false
    /// Float64.minPositive.isNormal;                // true
    /// (Float64.minPositive / 2.0).isNormal;        // false (subnormal)
    /// ```
    public var isNormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() >= Float32.minPositive
    }}

    /// True if `self` is subnormal (denormalized) — finite, non-zero, and
    /// smaller than `minPositive` in magnitude. Subnormals trade precision
    /// for range near zero.
    public var isSubnormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() < Float32.minPositive
    }}

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Sign as a float: `-1.0`, `0.0`, or `1.0`. NaN propagates as NaN.
    /// Negative zero returns `-0.0` (which still compares equal to `0.0`).
    ///
    /// # Examples
    ///
    /// ```
    /// (-3.14).sign;        // -1.0
    /// (0.0).sign;          //  0.0
    /// (3.14).sign;         //  1.0
    /// Float64.nan.sign;    //  nan
    /// ```
    public var sign: Float32 { get {
        if self.isNaN { Float32.nan }
        else if self.isZero {
            let one = Float32.one;
            let inverse = one.divide(self);
            if inverse < 0.0 {
                let zero = Float32.zero;
                zero.negate()
            } else {
                Float32(floatLiteral: 0.0)
            }
        }
        else if self < 0.0 { Float32(raw: lang.f32_neg(1.0)) }
        else { Float32(floatLiteral: 1.0) }
    }}

    /// True if `self > 0.0`. False for `+0.0`, `-0.0`, `nan`, and negatives.
    public var isPositive: Bool { get {
        self > 0.0
    }}

    /// True if `self < 0.0`. False for `-0.0`, `nan`, zero, and positives.
    /// To detect signed zero specifically, use `sign`.
    public var isNegative: Bool { get {
        self < 0.0
    }}

    /// True if `self` is `+0.0` or `-0.0`. Both signed zeros compare equal.
    public var isZero: Bool { get {
        self == 0.0
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// IEEE 754 equality. NaN is not equal to itself; `+0.0` equals `-0.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.14).equals(3.14);                  // true
    /// (0.0).equals(-0.0);                   // true
    /// Float64.nan.equals(Float64.nan);      // false (!)
    /// ```
    public func equals(other: Float32) -> Bool {
        Bool(boolLiteral: lang.f32_eq(self.raw, other.raw))
    }

    /// Three-way comparison returning an `Ordering`. NaN is *not* an ordered
    /// value — comparisons against NaN currently fall through to `.Equal`,
    /// which is wrong; gate inputs with `isNaN` if you need a well-defined
    /// answer.
    ///
    /// # Examples
    ///
    /// ```
    /// (1.0).compare(2.0);              // .Less
    /// (2.0).compare(2.0);              // .Equal
    /// (3.0).compare(2.0);              // .Greater
    /// (1.0).compare(Float64.infinity); // .Less
    /// ```
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

    /// IEEE 754 addition. NaN propagates; `inf + (-inf)` is NaN; finite + inf
    /// is inf.
    public func add(other: Float32) -> Float32 { Float32(raw: lang.f32_add(self.raw, other.raw)) }

    /// IEEE 754 subtraction. `inf - inf` is NaN; otherwise mirrors `add`.
    public func subtract(other: Float32) -> Float32 { Float32(raw: lang.f32_sub(self.raw, other.raw)) }

    /// IEEE 754 multiplication. NaN propagates; `inf * 0` is NaN; sign of
    /// the result follows the usual algebra.
    public func multiply(other: Float32) -> Float32 { Float32(raw: lang.f32_mul(self.raw, other.raw)) }

    /// IEEE 754 division. Unlike integer divide, dividing by zero does not
    /// trap — it produces ±infinity (or NaN for `0.0 / 0.0`).
    ///
    /// Special cases:
    /// - `x / 0.0` → `+inf` if `x > 0`, `-inf` if `x < 0`, `nan` if `x == 0`
    /// - `x / inf` → 0 for finite `x`
    /// - `inf / inf` → NaN
    ///
    /// # Examples
    ///
    /// ```
    /// (10.0).divide(4.0);  // 2.5
    /// 1.0 / 0.0;                  // inf
    /// 0.0 / 0.0;                  // nan
    /// ```
    public func divide(other: Float32) -> Float32 { Float32(raw: lang.f32_div(self.raw, other.raw)) }

    /// IEEE 754 negation — flips the sign bit. `-nan` is still NaN; `-(-0.0)`
    /// is `+0.0`.
    public func negate() -> Float32 { Float32(raw: lang.f32_neg(self.raw)) }

    // ========================================================================
    // BASIC MATHEMATICAL FUNCTIONS
    // ========================================================================

    /// Absolute value — clears the sign bit. NaN stays NaN; `-0.0` becomes
    /// `+0.0`.
    public func abs() -> Float32 {
        if Bool(boolLiteral: lang.f32_lt(self.raw, 0.0)) { self.negate() } else { self }
    }

    /// Largest integer ≤ `self`. Rounds toward `-infinity`.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.7).floor();   //  3.0
    /// (-3.2).floor();  // -4.0
    /// ```
    public func floor() -> Float32 { Float32(raw: lang.f32_floor(self.raw)) }

    /// Smallest integer ≥ `self`. Rounds toward `+infinity`.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.2).ceil();   //  4.0
    /// (-3.7).ceil();  // -3.0
    /// ```
    public func ceil() -> Float32 { Float32(raw: lang.f32_ceil(self.raw)) }

    /// Round to nearest integer, breaking ties *away from zero* (banker's
    /// rounding is not used).
    ///
    /// # Examples
    ///
    /// ```
    /// (3.4).round();   //  3.0
    /// (3.5).round();   //  4.0   (tie → away from zero)
    /// (-3.5).round();  // -4.0
    /// ```
    public func round() -> Float32 { Float32(raw: lang.f32_round(self.raw)) }

    /// Integer part, truncating toward zero. `floor` for positives, `ceil`
    /// for negatives.
    public func trunc() -> Float32 { Float32(raw: lang.f32_trunc(self.raw)) }

    /// Fractional part — `self - self.trunc()`. Sign matches `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.7).fract();   //  0.7
    /// (-3.7).fract();  // -0.7
    /// ```
    public func fract() -> Float32 {
        self.subtract(self.trunc())
    }

    /// Principal square root. Negatives return NaN (`-0.0` returns `-0.0`);
    /// `+inf` returns `+inf`.
    ///
    /// # Examples
    ///
    /// ```
    /// (4.0).sqrt();    // 2.0
    /// (-1.0).sqrt();   // nan
    /// ```
    public func sqrt() -> Float32 { Float32(raw: lang.f32_sqrt(self.raw)) }

    /// Real cube root. Defined for negatives — `(-8.0).cbrt() == -2.0`.
    public func cbrt() -> Float32 {
        Float32(raw: libm_cbrtf(self.raw))
    }

    /// Hypotenuse — `sqrt(self² + other²)`, computed via libm in a way that
    /// avoids intermediate overflow when one operand is very large.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.0).hypot(4.0);  // 5.0
    /// ```
    public func hypot(other: Float32) -> Float32 {
        Float32(raw: libm_hypotf(self.raw, other.raw))
    }

    // ========================================================================
    // EXPONENTIAL AND LOGARITHMIC FUNCTIONS
    // ========================================================================

    /// `e^self` via libm. `(-inf).exp()` is `0.0`; `(inf).exp()` is `inf`.
    public func exp() -> Float32 { Float32(raw: libm_expf(self.raw)) }

    /// `2^self`. Useful for binary scaling.
    public func exp2() -> Float32 { Float32(raw: libm_exp2f(self.raw)) }

    /// `e^self - 1`, computed without the cancellation that hurts
    /// `self.exp() - 1.0` for small `self`.
    public func expm1() -> Float32 { Float32(raw: libm_expm1f(self.raw)) }

    /// Natural logarithm. Negatives return NaN; zero returns `-inf`.
    ///
    /// # Examples
    ///
    /// ```
    /// (1.0).ln();           //  0.0
    /// Float64.e.ln();       //  1.0
    /// (0.0).ln();           // -inf
    /// (-1.0).ln();          //  nan
    /// ```
    public func ln() -> Float32 { Float32(raw: libm_logf(self.raw)) }

    /// `ln(1 + self)`, accurate for small `self` where `(1.0 + self).ln()`
    /// would lose digits.
    public func ln1p() -> Float32 { Float32(raw: libm_log1pf(self.raw)) }

    /// Base-2 logarithm.
    public func log2() -> Float32 { Float32(raw: libm_log2f(self.raw)) }

    /// Base-10 logarithm.
    public func log10() -> Float32 { Float32(raw: libm_log10f(self.raw)) }

    /// Logarithm with arbitrary base, computed as `self.ln() / base.ln()`.
    /// Pick `log2` or `log10` directly when you can — they avoid the second
    /// libm call.
    public func log(base: Float32) -> Float32 {
        self.ln().divide(base.ln())
    }

    /// `self ^ exponent` via libm. Negative bases with non-integer exponents
    /// return NaN.
    ///
    /// # Examples
    ///
    /// ```
    /// (2.0).pow(10.0);   // 1024.0
    /// (2.0).pow(0.5);    // sqrt(2)
    /// (-2.0).pow(0.5);   // nan
    /// ```
    public func pow(exponent: Float32) -> Float32 {
        Float32(raw: libm_powf(self.raw, exponent.raw))
    }

    /// Integer-exponent power via repeated squaring. Faster and more accurate
    /// than `pow` when the exponent is known to be integral. Negative
    /// exponents invert.
    ///
    /// # Examples
    ///
    /// ```
    /// (2.0).powi(10);   // 1024.0
    /// (2.0).powi(-1);   // 0.5
    /// ```
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

    /// Sine of `self` in radians.
    public func sin() -> Float32 { Float32(raw: libm_sinf(self.raw)) }

    /// Cosine of `self` in radians.
    public func cos() -> Float32 { Float32(raw: libm_cosf(self.raw)) }

    /// Tangent of `self` in radians. Diverges to ±large near `π/2 + kπ`.
    public func tan() -> Float32 { Float32(raw: libm_tanf(self.raw)) }

    /// Arc sine, result in radians on `[-π/2, π/2]`. Returns NaN outside
    /// `[-1.0, 1.0]`.
    public func asin() -> Float32 { Float32(raw: libm_asinf(self.raw)) }

    /// Arc cosine, result in radians on `[0, π]`. Returns NaN outside
    /// `[-1.0, 1.0]`.
    public func acos() -> Float32 { Float32(raw: libm_acosf(self.raw)) }

    /// Arc tangent, result in radians on `[-π/2, π/2]`. For full-quadrant
    /// recovery use `atan2`.
    public func atan() -> Float32 { Float32(raw: libm_atanf(self.raw)) }

    /// Two-argument arctangent — angle of the point `(x, self)` measured
    /// from the positive x-axis, on `[-π, π]`. Disambiguates quadrant where
    /// `atan` cannot.
    ///
    /// # Examples
    ///
    /// ```
    /// (1.0).atan2(1.0);    //  π/4   (Q1)
    /// (1.0).atan2(-1.0);   //  3π/4  (Q2)
    /// (-1.0).atan2(-1.0);  // -3π/4  (Q3)
    /// (-1.0).atan2(1.0);   // -π/4   (Q4)
    /// ```
    public func atan2(x: Float32) -> Float32 { Float32(raw: libm_atan2f(self.raw, x.raw)) }

    /// Sine and cosine in one call. Implemented via two libm calls today;
    /// kept for ergonomics and as a future optimisation point.
    ///
    /// # Examples
    ///
    /// ```
    /// let (s, c) = angle.sinCos();
    /// ```
    public func sinCos() -> (Float32, Float32) {
        (self.sin(), self.cos())
    }

    // ========================================================================
    // HYPERBOLIC FUNCTIONS
    // ========================================================================

    /// Hyperbolic sine.
    public func sinh() -> Float32 { Float32(raw: libm_sinhf(self.raw)) }

    /// Hyperbolic cosine.
    public func cosh() -> Float32 { Float32(raw: libm_coshf(self.raw)) }

    /// Hyperbolic tangent. Saturates at ±1 for large magnitudes.
    public func tanh() -> Float32 { Float32(raw: libm_tanhf(self.raw)) }

    /// Inverse hyperbolic sine. Defined on all real inputs.
    public func asinh() -> Float32 { Float32(raw: libm_asinhf(self.raw)) }

    /// Inverse hyperbolic cosine. Returns NaN for inputs less than `1.0`.
    public func acosh() -> Float32 { Float32(raw: libm_acoshf(self.raw)) }

    /// Inverse hyperbolic tangent. NaN outside `(-1.0, 1.0)`; `±inf` at ±1.
    public func atanh() -> Float32 { Float32(raw: libm_atanhf(self.raw)) }

    // ========================================================================
    // IEEE 754 OPERATIONS
    // ========================================================================

    /// Fused multiply-add — `(self * a) + b` with a single rounding step.
    /// More accurate (and often faster) than separate `multiply`/`add`.
    ///
    /// # Examples
    ///
    /// ```
    /// (2.0).fma(3.0, 4.0);   // 10.0
    /// ```
    public func fma(a: Float32, b: Float32) -> Float32 {
        Float32(raw: lang.f32_fma(self.raw, a.raw, b.raw))
    }

    /// Returns a value with `self`'s magnitude and `other`'s sign — i.e. an
    /// IEEE 754 `copysign`. Useful for unbiased rounding tricks.
    public func copysign(from other: Float32) -> Float32 {
        Float32(raw: lang.f32_copysign(self.raw, other.raw))
    }

    /// Next representable value greater than `self`. `+inf` and `nan` are
    /// fixed points; the largest finite value steps up to `+inf`.
    public func nextUp() -> Float32 {
        Float32(raw: libm_nextafterf(self.raw, lang.f32_infinity()))
    }

    /// Next representable value less than `self`. Mirror of `nextUp`.
    public func nextDown() -> Float32 {
        Float32(raw: libm_nextafterf(self.raw, lang.f32_neg(lang.f32_infinity())))
    }

    /// IEEE 754 remainder — uses round-to-nearest division, not truncation.
    /// Differs from `%`: `(5.0).remainder(dividingBy: 3.0)` is `-1.0`, not
    /// `2.0`.
    public func remainder(dividingBy other: Float32) -> Float32 {
        Float32(raw: libm_remainderf(self.raw, other.raw))
    }

    // ========================================================================
    // CLAMPING AND INTERPOLATION
    // ========================================================================

    /// Clamps `self` into `[min, max]`. NaN passes through unchanged. Caller
    /// must ensure `min <= max`.
    ///
    /// # Examples
    ///
    /// ```
    /// (0.5).clamp(0.0, 1.0);   // 0.5
    /// (-0.5).clamp(0.0, 1.0);  // 0.0
    /// (1.5).clamp(0.0, 1.0);   // 1.0
    /// ```
    public func clamp(min: Float32, max: Float32) -> Float32 {
        if self.isNaN { self }
        else if self < min { min }
        else if self > max { max }
        else { self }
    }

    /// Linear interpolation — `self + (other - self) * t`. `t == 0` returns
    /// `self`, `t == 1` returns `other`; `t` outside `[0, 1]` extrapolates.
    ///
    /// # Examples
    ///
    /// ```
    /// (0.0).lerp(to: 10.0, 0.0);   //  0.0
    /// (0.0).lerp(to: 10.0, 0.5);   //  5.0
    /// (0.0).lerp(to: 10.0, 1.0);   // 10.0
    /// (0.0).lerp(to: 10.0, 0.25);  //  2.5
    /// ```
    public func lerp(to other: Float32, t: Float32) -> Float32 {
        self.add(other.subtract(self).multiply(t))
    }

    // ========================================================================
    // CONVERSION
    // ========================================================================

    /// Truncates toward zero into an `Int64`. Returns `None` for NaN,
    /// infinity, or values that fall outside the `Int64` range.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.7).toInt64();              // Some(3)
    /// (-3.7).toInt64();             // Some(-3)
    /// Float64.nan.toInt64();        // None
    /// Float64.infinity.toInt64();   // None
    /// ```
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

    /// Converts to the sibling float type. Widening (32→64) is exact;
    /// narrowing (64→32) rounds and may overflow to ±infinity.
    public func toFloat64() -> Float64 {
        Float64(raw: lang.cast_f32_f64(self.raw))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    /// Parses a `Float32` from a string. Recognises decimal
    /// (`"3.14"`), scientific (`"1.5e10"`, `"2.5E-3"`), and the special
    /// tokens `"inf"`, `"-inf"`, `"+inf"`, `"infinity"`, `"nan"`
    /// (case-insensitive). Returns `None` for any other input.
    ///
    /// # Examples
    ///
    /// ```
    /// Float32.parse("3.14");      // Some(3.14)
    /// Float32.parse("-2.5e10");   // Some(-2.5e10)
    /// Float32.parse("inf");       // Some(infinity)
    /// Float32.parse("nan");       // Some(nan)
    /// Float32.parse("abc");       // None
    /// Float32.parse("");          // None
    /// ```
    public static func parse(string: String) -> Float32? {
        let len = string.byteCount;
        if len == 0 {
            return .None
        }

        // Check for special values
        // "nan"
        if len == 3 {
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            // 'n' or 'N' = 110 or 78
            // 'a' or 'A' = 97 or 65
            let isN0 = Int64(from: b0) == 110 or Int64(from: b0) == 78;
            let isA1 = Int64(from: b1) == 97 or Int64(from: b1) == 65;
            let isN2 = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            if isN0 and isA1 and isN2 {
                return .Some(Float32.nan)
            }
        }

        // "inf"
        if len == 3 {
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            // 'i' or 'I' = 105 or 73
            // 'n' or 'N' = 110 or 78
            // 'f' or 'F' = 102 or 70
            let isI = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            if isI and isN and isF {
                return .Some(Float32.infinity)
            }
        }

        // "-inf"
        if len == 4 {
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let b3: UInt8 = string.bytes(unchecked: 3);
            let isMinus = Int64(from: b0) == 45;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isMinus and isI and isN and isF {
                return .Some(Float32(raw: lang.f32_neg(lang.f32_infinity())))
            }
        }

        // "+inf"
        if len == 4 {
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let b3: UInt8 = string.bytes(unchecked: 3);
            let isPlus = Int64(from: b0) == 43;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isPlus and isI and isN and isF {
                return .Some(Float32.infinity)
            }
        }

        // "infinity"
        if len == 8 {
            // Check for "infinity" (case insensitive)
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let b3: UInt8 = string.bytes(unchecked: 3);
            let b4: UInt8 = string.bytes(unchecked: 4);
            let b5: UInt8 = string.bytes(unchecked: 5);
            let b6: UInt8 = string.bytes(unchecked: 6);
            let b7: UInt8 = string.bytes(unchecked: 7);
            let isI0 = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN1 = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF2 = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            let isI3 = Int64(from: b3) == 105 or Int64(from: b3) == 73;
            let isN4 = Int64(from: b4) == 110 or Int64(from: b4) == 78;
            let isI5 = Int64(from: b5) == 105 or Int64(from: b5) == 73;
            let isT6 = Int64(from: b6) == 116 or Int64(from: b6) == 84;
            let isY7 = Int64(from: b7) == 121 or Int64(from: b7) == 89;
            if isI0 and isN1 and isF2 and isI3 and isN4 and isI5 and isT6 and isY7 {
                return .Some(Float32.infinity)
            }
        }

        // Parse regular number: [+-]?[0-9]*[.]?[0-9]*([eE][+-]?[0-9]+)?
        var index: Int64 = 0;
        var isNegative = false;

        // Check for sign
        let firstByte: UInt8 = string.bytes(unchecked: 0);
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
        var integerPart: Float32 = 0.0;
        var hasIntegerPart = false;
        var currentByte: Int64 = Int64(from: string.bytes(unchecked: index));

        while index < len and currentByte >= 48 and currentByte <= 57 {
            let digit = Float32(from: currentByte - 48);
            integerPart = integerPart * 10.0 + digit;
            hasIntegerPart = true;
            index = index + 1;
            if index < len {
                currentByte = Int64(from: string.bytes(unchecked: index))
            }
        }

        // Parse fractional part
        var fractionalPart: Float32 = 0.0;
        var hasFractionalPart = false;

        if index < len and currentByte == 46 {  // '.'
            index = index + 1;
            var divisor: Float32 = 10.0;

            if index < len {
                currentByte = Int64(from: string.bytes(unchecked: index));
                while index < len and currentByte >= 48 and currentByte <= 57 {
                    let digit = Float32(from: currentByte - 48);
                    fractionalPart = fractionalPart + digit / divisor;
                    divisor = divisor * 10.0;
                    hasFractionalPart = true;
                    index = index + 1;
                    if index < len {
                        currentByte = Int64(from: string.bytes(unchecked: index))
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
            currentByte = Int64(from: string.bytes(unchecked: index));

            if currentByte == 45 {  // '-'
                expNegative = true;
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.bytes(unchecked: index))
                }
            } else if currentByte == 43 {  // '+'
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.bytes(unchecked: index))
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
                    currentByte = Int64(from: string.bytes(unchecked: index))
                }
            }

            if not hasExpDigit {
                return .None
            }

            // Apply exponent using pow
            let expFloat = Float32(from: exponent);
            let ten: Float32 = 10.0;
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

    /// Renders the float to a `String`, honouring the supplied
    /// `FormatOptions`. Implements `Formattable`.
    ///
    /// Recognised options:
    /// - `precision` — digits after the decimal point (default 6).
    /// - `width` / `fill` / `alignment` — padding control.
    /// - `sign` — `.Negative` (default), `.Always`, or `.Space`.
    /// - `floatStyle` — `.Fixed`, `.Scientific`, `.Auto`, or `.Percent`.
    ///   `.Auto` picks fixed or scientific based on magnitude.
    ///   `.Percent` multiplies by 100 and appends `%`.
    ///
    /// String interpolation forwards through the same options:
    /// `"\{x:.2}"` is two decimal places, `"\{x:.2e}"` is scientific,
    /// `"\{x:%}"` is percentage.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.14159).format();                                          // "3.14159"
    /// (3.14159).format(.{precision: 2});                  // "3.14"
    /// (1234.5).format(.{floatStyle: .Scientific});        // "1.2345e3"
    /// (0.756).format(.{floatStyle: .Percent});            // "75.6%"
    /// (3.14).format(.{width: 8, fill: '0'});              // "00003.14"
    /// (3.14).format(.{sign: .Always});                    // "+3.14"
    /// ```
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
                let one = Float32.one;
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
                    let expInt: Int64 = Int64(raw: lang.cast_f32_i64(expVal.raw));
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
                    exponent = Int64(raw: lang.cast_f32_i64(expVal.raw));
                    let pow10 = Float32(floatLiteral: 10.0).powi(exponent);
                    mantissa = value.divide(pow10);
                }

                let scale = Float32(floatLiteral: 10.0).powi(precision);
                mantissa = mantissa.multiply(scale).round().divide(scale);
                if mantissa >= 10.0 {
                    mantissa = mantissa.divide(10.0);
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
                        number.appendByte(digits.bytes(unchecked: i));
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
                        number.appendByte(digits.bytes(unchecked: i));
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
                    rounded = rounded.multiply(scale).round().divide(scale)
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
                        number.appendByte(digits.bytes(unchecked: i));
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
                let b = number.bytes(unchecked: i);
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
                    let b = number.bytes(unchecked: trimEnd - 1);
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

