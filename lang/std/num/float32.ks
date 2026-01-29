// Float32 - 32-bit floating point
// Generated from float.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Formattable,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral, Convertible
)
import std.text.(String)
import std.num.(Int64, Float64)

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
    Convertible[Int64],
    Convertible[Float64],
    FFISafe
{
    public var raw: lang.f32

    // ========================================================================
    // CONSTANTS - Basic Values
    // ========================================================================

    public static var zero: Float32 { Float32(floatLiteral: 0.0) }
    public static var one: Float32 { Float32(floatLiteral: 1.0) }
    public static var minValue: Float32 { Float32(floatLiteral: 3.4028235e38).negate() }
    public static var maxValue: Float32 { Float32(floatLiteral: 3.4028235e38) }
    public static var minPositive: Float32 { Float32(floatLiteral: 1.17549435e-38) }
    public static var epsilon: Float32 { Float32(floatLiteral: 1.1920929e-7) }

    // ========================================================================
    // CONSTANTS - Special Values
    // ========================================================================

    public static var infinity: Float32 { Float32(raw: lang.f32_infinity()) }
    public static var nan: Float32 { Float32(raw: lang.f32_nan()) }

    // ========================================================================
    // CONSTANTS - Mathematical
    // ========================================================================

    public static var pi: Float32 { Float32(floatLiteral: 3.141592653589793) }
    public static var e: Float32 { Float32(floatLiteral: 2.718281828459045) }
    public static var tau: Float32 { Float32(floatLiteral: 6.283185307179586) }
    public static var ln2: Float32 { Float32(floatLiteral: 0.6931471805599453) }
    public static var ln10: Float32 { Float32(floatLiteral: 2.302585092994046) }
    public static var sqrt2: Float32 { Float32(floatLiteral: 1.4142135623730951) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    public init(floatLiteral value: lang.f64) {
        self.raw = lang.cast_f64_f32(value)
    }

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f32(value)
    }

    init(raw value: lang.f32) {
        self.raw = value
    }

    public init(from value: Int64) {
        self.raw = lang.cast_i64_f32(value.raw)
    }

    public init(from value: Float64) {
        self.raw = lang.cast_f64_f32(value.raw)
    }

    // ========================================================================
    // CLASSIFICATION (Properties)
    // ========================================================================

    public var isNaN: Bool { get {
        Bool(boolLiteral: lang.f32_is_nan(self.raw))
    }}

    public var isInfinite: Bool { get {
        Bool(boolLiteral: lang.f32_is_infinite(self.raw))
    }}

    public var isFinite: Bool { get {
        not self.isNaN and not self.isInfinite
    }}

    // TODO: requires lang.f32_is_normal intrinsic
    public var isNormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() >= Float32.minPositive
    }}

    // TODO: requires lang.f32_is_subnormal intrinsic
    public var isSubnormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() < Float32.minPositive
    }}

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: Float32 { get {
        if self.isNaN { Float32.nan }
        else if self < 0.0 { Float32(raw: lang.f32_neg(1.0)) }
        else if self > 0.0 { Float32(floatLiteral: 1.0) }
        else { Float32(floatLiteral: 0.0) }
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

    public func equals(other: Float32) -> Bool {
        Bool(boolLiteral: lang.f32_eq(self.raw, other.raw))
    }

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

    public func add(other: Float32) -> Float32 { Float32(raw: lang.f32_add(self.raw, other.raw)) }
    public func subtract(other: Float32) -> Float32 { Float32(raw: lang.f32_sub(self.raw, other.raw)) }
    public func multiply(other: Float32) -> Float32 { Float32(raw: lang.f32_mul(self.raw, other.raw)) }
    public func divide(other: Float32) -> Float32 { Float32(raw: lang.f32_div(self.raw, other.raw)) }
    public func negate() -> Float32 { Float32(raw: lang.f32_neg(self.raw)) }

    // ========================================================================
    // BASIC MATHEMATICAL FUNCTIONS
    // ========================================================================

    public func abs() -> Float32 {
        if Bool(boolLiteral: lang.f32_lt(self.raw, 0.0)) { self.negate() } else { self }
    }

    public func floor() -> Float32 { Float32(raw: lang.f32_floor(self.raw)) }
    public func ceil() -> Float32 { Float32(raw: lang.f32_ceil(self.raw)) }
    public func round() -> Float32 { Float32(raw: lang.f32_round(self.raw)) }
    public func trunc() -> Float32 { Float32(raw: lang.f32_trunc(self.raw)) }

    public func fract() -> Float32 {
        self.subtract(self.trunc())
    }

    public func sqrt() -> Float32 { Float32(raw: lang.f32_sqrt(self.raw)) }

    // TODO: requires lang.f32_cbrt intrinsic
    public func cbrt() -> Float32 {
        // Stub: cube root not available, return nan
        Float32.nan
    }

    // TODO: requires lang.f32_hypot intrinsic
    public func hypot(other: Float32) -> Float32 {
        // Naive implementation (may overflow for large values)
        (self.multiply(self).add(other.multiply(other))).sqrt()
    }

    // ========================================================================
    // EXPONENTIAL AND LOGARITHMIC FUNCTIONS
    // ========================================================================

    // TODO: requires lang.f32_exp intrinsic
    public func exp() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_exp2 intrinsic
    public func exp2() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_expm1 intrinsic
    public func expm1() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_ln intrinsic
    public func ln() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_ln1p intrinsic
    public func ln1p() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_log2 intrinsic
    public func log2() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_log10 intrinsic
    public func log10() -> Float32 { Float32.nan }

    // TODO: requires ln intrinsic
    public func log(base: Float32) -> Float32 { Float32.nan }

    // TODO: requires lang.f32_pow intrinsic
    public func pow(exponent: Float32) -> Float32 { Float32.nan }

    // TODO: requires lang.f32_powi intrinsic
    public func powi(exponent: Int64) -> Float32 {
        // Simple implementation for integer powers
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

    // TODO: requires lang.f32_sin intrinsic
    public func sin() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_cos intrinsic
    public func cos() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_tan intrinsic
    public func tan() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_asin intrinsic
    public func asin() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_acos intrinsic
    public func acos() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_atan intrinsic
    public func atan() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_atan2 intrinsic
    public func atan2(x: Float32) -> Float32 { Float32.nan }

    // TODO: requires sin and cos intrinsics
    public func sinCos() -> (Float32, Float32) {
        (self.sin(), self.cos())
    }

    // ========================================================================
    // HYPERBOLIC FUNCTIONS
    // ========================================================================

    // TODO: requires lang.f32_sinh intrinsic
    public func sinh() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_cosh intrinsic
    public func cosh() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_tanh intrinsic
    public func tanh() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_asinh intrinsic
    public func asinh() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_acosh intrinsic
    public func acosh() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_atanh intrinsic
    public func atanh() -> Float32 { Float32.nan }

    // ========================================================================
    // IEEE 754 OPERATIONS
    // ========================================================================

    // TODO: requires lang.f32_fma intrinsic
    public func fma(a: Float32, b: Float32) -> Float32 {
        // Naive implementation without FMA
        self.multiply(a).add(b)
    }

    // TODO: requires lang.f32_copysign intrinsic
    public func copysign(from other: Float32) -> Float32 {
        let magnitude = self.abs();
        if other < 0.0 { magnitude.negate() } else { magnitude }
    }

    // TODO: requires lang.f32_nextUp intrinsic
    public func nextUp() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_nextDown intrinsic
    public func nextDown() -> Float32 { Float32.nan }

    // TODO: requires lang.f32_remainder intrinsic (IEEE 754)
    public func remainder(dividingBy other: Float32) -> Float32 { Float32.nan }

    // ========================================================================
    // CLAMPING AND INTERPOLATION
    // ========================================================================

    public func clamp(min: Float32, max: Float32) -> Float32 {
        if self.isNaN { self }
        else if self < min { min }
        else if self > max { max }
        else { self }
    }

    public func lerp(to other: Float32, t: Float32) -> Float32 {
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
        .Some(Int64(raw: lang.cast_f32_i64(self.raw)))
    }

    public func toFloat64() -> Float64 {
        Float64(raw: lang.cast_f32_f64(self.raw))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> Float32?

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
        var intVal: Int64 = Int64(raw: lang.cast_f32_i64(intPart.raw));

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
        let ten: Float32 = 10.0;

        while digitCount < maxDigits {
            fracPart = fracPart * ten;
            let digit: Int64 = Int64(raw: lang.cast_f32_i64(fracPart.trunc().raw));
            let charCode: Int64 = digit + 48;
            result.appendByte(UInt8(from: charCode));
            fracPart = fracPart - Float32(raw: lang.cast_i64_f32(digit.raw));
            digitCount = digitCount + 1
        }

        result
    }}

