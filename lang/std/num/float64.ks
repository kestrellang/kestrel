// Float64 - 64-bit floating point
// Generated from float.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Formattable,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral, Convertible
)
import std.text.(String)
import std.num.(Int64, Float32)

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
    Convertible[Int64],
    Convertible[Float32],
    FFISafe
{
    public var raw: lang.f64

    // ========================================================================
    // CONSTANTS - Basic Values
    // ========================================================================

    public static var zero: Float64 { Float64(floatLiteral: 0.0) }
    public static var one: Float64 { Float64(floatLiteral: 1.0) }
    public static var minValue: Float64 { Float64(floatLiteral: 1.7976931348623157e308).negate() }
    public static var maxValue: Float64 { Float64(floatLiteral: 1.7976931348623157e308) }
    public static var minPositive: Float64 { Float64(floatLiteral: 2.2250738585072014e-308) }
    public static var epsilon: Float64 { Float64(floatLiteral: 2.220446049250313e-16) }

    // ========================================================================
    // CONSTANTS - Special Values
    // ========================================================================

    public static var infinity: Float64 { Float64(raw: lang.f64_infinity()) }
    public static var nan: Float64 { Float64(raw: lang.f64_nan()) }

    // ========================================================================
    // CONSTANTS - Mathematical
    // ========================================================================

    public static var pi: Float64 { Float64(floatLiteral: 3.141592653589793) }
    public static var e: Float64 { Float64(floatLiteral: 2.718281828459045) }
    public static var tau: Float64 { Float64(floatLiteral: 6.283185307179586) }
    public static var ln2: Float64 { Float64(floatLiteral: 0.6931471805599453) }
    public static var ln10: Float64 { Float64(floatLiteral: 2.302585092994046) }
    public static var sqrt2: Float64 { Float64(floatLiteral: 1.4142135623730951) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    public init(floatLiteral value: lang.f64) {
        self.raw = value
    }

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f64(value)
    }

    init(raw value: lang.f64) {
        self.raw = value
    }

    public init(from value: Int64) {
        self.raw = lang.cast_i64_f64(value.raw)
    }

    public init(from value: Float32) {
        self.raw = lang.cast_f32_f64(value.raw)
    }

    // ========================================================================
    // CLASSIFICATION (Properties)
    // ========================================================================

    public var isNaN: Bool { get {
        Bool(boolLiteral: lang.f64_is_nan(self.raw))
    }}

    public var isInfinite: Bool { get {
        Bool(boolLiteral: lang.f64_is_infinite(self.raw))
    }}

    public var isFinite: Bool { get {
        not self.isNaN and not self.isInfinite
    }}

    // TODO: requires lang.f64_is_normal intrinsic
    public var isNormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() >= Float64.minPositive
    }}

    // TODO: requires lang.f64_is_subnormal intrinsic
    public var isSubnormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() < Float64.minPositive
    }}

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: Float64 { get {
        if self.isNaN { Float64.nan }
        else if self < 0.0 { Float64(raw: lang.f64_neg(1.0)) }
        else if self > 0.0 { Float64(floatLiteral: 1.0) }
        else { Float64(floatLiteral: 0.0) }
    }}

    public var isPositive: Bool { get {
        self > 0.0
    }}

    public var isNegative: Bool { get {
        self < 0.0
    }}

    public var isZero: Bool { get {
        self == 0.0
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    public func equals(other: Float64) -> Bool {
        Bool(boolLiteral: lang.f64_eq(self.raw, other.raw))
    }

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

    public func add(other: Float64) -> Float64 { Float64(raw: lang.f64_add(self.raw, other.raw)) }
    public func subtract(other: Float64) -> Float64 { Float64(raw: lang.f64_sub(self.raw, other.raw)) }
    public func multiply(other: Float64) -> Float64 { Float64(raw: lang.f64_mul(self.raw, other.raw)) }
    public func divide(other: Float64) -> Float64 { Float64(raw: lang.f64_div(self.raw, other.raw)) }
    public func negate() -> Float64 { Float64(raw: lang.f64_neg(self.raw)) }

    // ========================================================================
    // BASIC MATHEMATICAL FUNCTIONS
    // ========================================================================

    public func abs() -> Float64 {
        if Bool(boolLiteral: lang.f64_lt(self.raw, 0.0)) { self.negate() } else { self }
    }

    public func floor() -> Float64 { Float64(raw: lang.f64_floor(self.raw)) }
    public func ceil() -> Float64 { Float64(raw: lang.f64_ceil(self.raw)) }
    public func round() -> Float64 { Float64(raw: lang.f64_round(self.raw)) }
    public func trunc() -> Float64 { Float64(raw: lang.f64_trunc(self.raw)) }

    public func fract() -> Float64 {
        self.subtract(self.trunc())
    }

    public func sqrt() -> Float64 { Float64(raw: lang.f64_sqrt(self.raw)) }

    // TODO: requires lang.f64_cbrt intrinsic
    public func cbrt() -> Float64 {
        // Stub: cube root not available, return nan
        Float64.nan
    }

    // TODO: requires lang.f64_hypot intrinsic
    public func hypot(other: Float64) -> Float64 {
        // Naive implementation (may overflow for large values)
        (self.multiply(self).add(other.multiply(other))).sqrt()
    }

    // ========================================================================
    // EXPONENTIAL AND LOGARITHMIC FUNCTIONS
    // ========================================================================

    // TODO: requires lang.f64_exp intrinsic
    public func exp() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_exp2 intrinsic
    public func exp2() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_expm1 intrinsic
    public func expm1() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_ln intrinsic
    public func ln() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_ln1p intrinsic
    public func ln1p() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_log2 intrinsic
    public func log2() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_log10 intrinsic
    public func log10() -> Float64 { Float64.nan }

    // TODO: requires ln intrinsic
    public func log(base: Float64) -> Float64 { Float64.nan }

    // TODO: requires lang.f64_pow intrinsic
    public func pow(exponent: Float64) -> Float64 { Float64.nan }

    // TODO: requires lang.f64_powi intrinsic
    public func powi(exponent: Int64) -> Float64 {
        // Simple implementation for integer powers
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

    // TODO: requires lang.f64_sin intrinsic
    public func sin() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_cos intrinsic
    public func cos() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_tan intrinsic
    public func tan() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_asin intrinsic
    public func asin() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_acos intrinsic
    public func acos() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_atan intrinsic
    public func atan() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_atan2 intrinsic
    public func atan2(x: Float64) -> Float64 { Float64.nan }

    // TODO: requires sin and cos intrinsics
    public func sinCos() -> (Float64, Float64) {
        (self.sin(), self.cos())
    }

    // ========================================================================
    // HYPERBOLIC FUNCTIONS
    // ========================================================================

    // TODO: requires lang.f64_sinh intrinsic
    public func sinh() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_cosh intrinsic
    public func cosh() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_tanh intrinsic
    public func tanh() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_asinh intrinsic
    public func asinh() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_acosh intrinsic
    public func acosh() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_atanh intrinsic
    public func atanh() -> Float64 { Float64.nan }

    // ========================================================================
    // IEEE 754 OPERATIONS
    // ========================================================================

    // TODO: requires lang.f64_fma intrinsic
    public func fma(a: Float64, b: Float64) -> Float64 {
        // Naive implementation without FMA
        self.multiply(a).add(b)
    }

    // TODO: requires lang.f64_copysign intrinsic
    public func copysign(from other: Float64) -> Float64 {
        let magnitude = self.abs();
        if other < 0.0 { magnitude.negate() } else { magnitude }
    }

    // TODO: requires lang.f64_nextUp intrinsic
    public func nextUp() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_nextDown intrinsic
    public func nextDown() -> Float64 { Float64.nan }

    // TODO: requires lang.f64_remainder intrinsic (IEEE 754)
    public func remainder(dividingBy other: Float64) -> Float64 { Float64.nan }

    // ========================================================================
    // CLAMPING AND INTERPOLATION
    // ========================================================================

    public func clamp(min: Float64, max: Float64) -> Float64 {
        if self.isNaN { self }
        else if self < min { min }
        else if self > max { max }
        else { self }
    }

    public func lerp(to other: Float64, t: Float64) -> Float64 {
        self.add(other.subtract(self).multiply(t))
    }

    // ========================================================================
    // CONVERSION
    // ========================================================================

    // TODO: requires proper bounds checking
    public func toInt64() -> Int64? {
        if self.isNaN or self.isInfinite {
            return .None
        };
        .Some(Int64(raw: lang.cast_f64_i64(self.raw)))
    }

    public func toFloat32() -> Float32 {
        Float32(raw: lang.cast_f64_f32(self.raw))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> Float64?

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format() -> String {
        // Handle special cases
        if self.isNaN {
            return "NaN"
        }
        if self.isInfinite {
            if self < 0.0 {
                return "-Infinity"
            } else {
                return "Infinity"
            }
        }

        var result = String();
        var value = self;

        // Handle negative
        let isNegative = value < 0.0;
        if isNegative {
            result.appendByte(45);  // '-'
            value = value.negate()
        }

        // Get integer part
        let intPart = value.trunc();
        var intVal: Int64 = Int64(raw: lang.cast_f64_i64(intPart.raw));

        // Format integer part
        if intVal == 0 {
            result.appendByte(48)  // '0'
        } else {
            var digits = String();
            while intVal > 0 {
                let digit: Int64 = intVal % 10;
                let charCode: Int64 = digit + 48;
                digits.appendByte(UInt8(from: charCode));
                intVal = intVal / 10
            }
            // Reverse digits
            var i = digits.byteCount() - 1;
            while i >= 0 {
                result.appendByte(digits.byteAtUnchecked(i));
                i = i - 1
            }
        }

        // Add decimal point
        result.appendByte(46);  // '.'

        // Get fractional part (6 digits of precision)
        var fracPart = value - intPart;
        var digitCount: Int64 = 0;
        let maxDigits: Int64 = 6;
        let ten: Float64 = 10.0;

        while digitCount < maxDigits {
            fracPart = fracPart * ten;
            let digit: Int64 = Int64(raw: lang.cast_f64_i64(fracPart.trunc().raw));
            let charCode: Int64 = digit + 48;
            result.appendByte(UInt8(from: charCode));
            fracPart = fracPart - Float64(raw: lang.cast_i64_f64(digit.raw));
            digitCount = digitCount + 1
        }

        result
    }}

// Float - alias to Float64
public type Float = Float64
