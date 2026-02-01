// Float64 - 64-bit floating point
// Generated from float.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Formattable,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral, Convertible, Defaultable
)
import std.text.(String)
import std.num.(Int64, Float32)

/// A 64-bit IEEE 754 floating-point type.
///
/// Float64 supports arithmetic, comparison, mathematical functions, and formatting.
///
/// Special values:
/// - `Float64.nan`: Not-a-Number (result of undefined operations like 0/0)
/// - `Float64.infinity`: Positive infinity (result of overflow or 1/0)
///
/// NaN behavior:
/// - NaN is not equal to anything, including itself: `nan == nan` is false
/// - NaN comparisons always return false: `nan < 0`, `nan > 0` are both false
/// - Any arithmetic with NaN produces NaN
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
    public var raw: lang.f64

    // ========================================================================
    // CONSTANTS - Basic Values
    // ========================================================================

    /// The zero value (0.0).
    public static var zero: Float64 { Float64(floatLiteral: 0.0) }

    /// The one value (1.0).
    public static var one: Float64 { Float64(floatLiteral: 1.0) }

    /// The minimum finite value (most negative).
    public static var minValue: Float64 { Float64(floatLiteral: 1.7976931348623157e308).negate() }

    /// The maximum finite value (most positive).
    public static var maxValue: Float64 { Float64(floatLiteral: 1.7976931348623157e308) }

    /// The smallest positive normal value.
    /// Values smaller than this are subnormal (denormalized) and have reduced precision.
    public static var minPositive: Float64 { Float64(floatLiteral: 2.2250738585072014e-308) }

    /// Machine epsilon: the smallest value such that `1.0 + epsilon != 1.0`.
    /// Useful for comparing floating-point values with tolerance.
    public static var epsilon: Float64 { Float64(floatLiteral: 2.220446049250313e-16) }

    // ========================================================================
    // CONSTANTS - Special Values
    // ========================================================================

    /// Positive infinity. Result of dividing by zero or overflow.
    public static var infinity: Float64 { Float64(raw: lang.f64_infinity()) }

    /// Not-a-Number. Result of undefined operations like 0/0.
    /// NaN is not equal to anything, including itself.
    public static var nan: Float64 { Float64(raw: lang.f64_nan()) }

    // ========================================================================
    // CONSTANTS - Mathematical
    // ========================================================================

    /// Pi (π ≈ 3.14159...). The ratio of a circle's circumference to its diameter.
    public static var pi: Float64 { Float64(floatLiteral: 3.141592653589793) }

    /// Euler's number (e ≈ 2.71828...). Base of the natural logarithm.
    public static var e: Float64 { Float64(floatLiteral: 2.718281828459045) }

    /// Tau (τ = 2π ≈ 6.28318...). The ratio of a circle's circumference to its radius.
    public static var tau: Float64 { Float64(floatLiteral: 6.283185307179586) }

    /// Natural logarithm of 2 (ln(2) ≈ 0.693...).
    public static var ln2: Float64 { Float64(floatLiteral: 0.6931471805599453) }

    /// Natural logarithm of 10 (ln(10) ≈ 2.302...).
    public static var ln10: Float64 { Float64(floatLiteral: 2.302585092994046) }

    /// Square root of 2 (√2 ≈ 1.414...).
    public static var sqrt2: Float64 { Float64(floatLiteral: 1.4142135623730951) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates a Float64 from a floating-point literal.
    public init(floatLiteral value: lang.f64) {
        self.raw = value
    }

    /// Creates a Float64 with the default value (zero).
    public init() {
        self.init(floatLiteral: 0.0)
    }

    /// Creates a Float64 from an integer literal.
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f64(value)
    }

    /// Creates a Float64 from a raw `lang.f64` value.
    init(raw value: lang.f64) {
        self.raw = value
    }

    /// Creates a Float64 from an Int64.
    public init(from value: Int64) {
        self.raw = lang.cast_i64_f64(value.raw)
    }

    /// Creates a Float64 from a Float32.
    public init(from value: Float32) {
        self.raw = lang.cast_f32_f64(value.raw)
    }

    // ========================================================================
    // CLASSIFICATION (Properties)
    // ========================================================================

    /// Returns true if this value is NaN (Not-a-Number).
    public var isNaN: Bool { get {
        Bool(boolLiteral: lang.f64_is_nan(self.raw))
    }}

    /// Returns true if this value is positive or negative infinity.
    public var isInfinite: Bool { get {
        Bool(boolLiteral: lang.f64_is_infinite(self.raw))
    }}

    /// Returns true if this value is finite (not NaN and not infinite).
    public var isFinite: Bool { get {
        not self.isNaN and not self.isInfinite
    }}

    /// Returns true if this value is a normal number (not zero, subnormal, infinite, or NaN).
    public var isNormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() >= Float64.minPositive
    }}

    /// Returns true if this value is subnormal (denormalized).
    /// Subnormal numbers have reduced precision but allow gradual underflow.
    public var isSubnormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() < Float64.minPositive
    }}

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Returns -1.0 if negative, 0.0 if zero, 1.0 if positive, or NaN if NaN.
    public var sign: Float64 { get {
        if self.isNaN { Float64.nan }
        else if self < 0.0 { Float64(raw: lang.f64_neg(1.0)) }
        else if self > 0.0 { Float64(floatLiteral: 1.0) }
        else { Float64(floatLiteral: 0.0) }
    }}

    /// Returns true if this value is greater than zero.
    public var isPositive: Bool { get {
        self > 0.0
    }}

    /// Returns true if this value is less than zero.
    public var isNegative: Bool { get {
        self < 0.0
    }}

    /// Returns true if this value is zero (positive or negative).
    public var isZero: Bool { get {
        self == 0.0
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Compares two values for equality.
    /// Note: NaN is not equal to anything, including itself.
    public func equals(other: Float64) -> Bool {
        Bool(boolLiteral: lang.f64_eq(self.raw, other.raw))
    }

    /// Compares this value to another, returning an Ordering.
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

    /// Adds two values.
    public func add(other: Float64) -> Float64 { Float64(raw: lang.f64_add(self.raw, other.raw)) }

    /// Subtracts two values.
    public func subtract(other: Float64) -> Float64 { Float64(raw: lang.f64_sub(self.raw, other.raw)) }

    /// Multiplies two values.
    public func multiply(other: Float64) -> Float64 { Float64(raw: lang.f64_mul(self.raw, other.raw)) }

    /// Divides two values.
    public func divide(other: Float64) -> Float64 { Float64(raw: lang.f64_div(self.raw, other.raw)) }
    /// Negates this value.
    public func negate() -> Float64 { Float64(raw: lang.f64_neg(self.raw)) }

    // ========================================================================
    // BASIC MATHEMATICAL FUNCTIONS
    // ========================================================================

    /// Returns the absolute value.
    public func abs() -> Float64 {
        if Bool(boolLiteral: lang.f64_lt(self.raw, 0.0)) { self.negate() } else { self }
    }

    /// Rounds down to the nearest integer.
    public func floor() -> Float64 { Float64(raw: lang.f64_floor(self.raw)) }

    /// Rounds up to the nearest integer.
    public func ceil() -> Float64 { Float64(raw: lang.f64_ceil(self.raw)) }

    /// Rounds to the nearest integer (half away from zero).
    public func round() -> Float64 { Float64(raw: lang.f64_round(self.raw)) }

    /// Truncates toward zero.
    public func trunc() -> Float64 { Float64(raw: lang.f64_trunc(self.raw)) }

    /// Returns the fractional part (self - trunc(self)).
    public func fract() -> Float64 {
        self.subtract(self.trunc())
    }

    /// Returns the square root.
    public func sqrt() -> Float64 { Float64(raw: lang.f64_sqrt(self.raw)) }

    /// Returns the cube root.
    public func cbrt() -> Float64 {
        Float64(raw: libm_cbrt(self.raw))
    }

    /// Returns the hypotenuse: sqrt(self² + other²).
    public func hypot(other: Float64) -> Float64 {
        Float64(raw: libm_hypot(self.raw, other.raw))
    }

    // ========================================================================
    // EXPONENTIAL AND LOGARITHMIC FUNCTIONS
    // ========================================================================

    /// Returns e^self (exponential).
    public func exp() -> Float64 { Float64(raw: libm_exp(self.raw)) }

    /// Returns 2^self.
    public func exp2() -> Float64 { Float64(raw: libm_exp2(self.raw)) }

    /// Returns e^self - 1, accurate for small values.
    public func expm1() -> Float64 { Float64(raw: libm_expm1(self.raw)) }

    /// Returns the natural logarithm (base e).
    public func ln() -> Float64 { Float64(raw: libm_log(self.raw)) }

    /// Returns ln(1 + self), accurate for small values.
    public func ln1p() -> Float64 { Float64(raw: libm_log1p(self.raw)) }

    /// Returns the base-2 logarithm.
    public func log2() -> Float64 { Float64(raw: libm_log2(self.raw)) }

    /// Returns the base-10 logarithm.
    public func log10() -> Float64 { Float64(raw: libm_log10(self.raw)) }

    /// Returns the logarithm with the given base.
    public func log(base: Float64) -> Float64 {
        self.ln().divide(base.ln())
    }

    /// Raises self to the given floating-point power.
    public func pow(exponent: Float64) -> Float64 {
        Float64(raw: libm_pow(self.raw, exponent.raw))
    }

    /// Raises self to the given integer power.
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
    public func sin() -> Float64 { Float64(raw: libm_sin(self.raw)) }

    /// Returns the cosine (argument in radians).
    public func cos() -> Float64 { Float64(raw: libm_cos(self.raw)) }

    /// Returns the tangent (argument in radians).
    public func tan() -> Float64 { Float64(raw: libm_tan(self.raw)) }

    /// Returns the arc sine (result in radians).
    public func asin() -> Float64 { Float64(raw: libm_asin(self.raw)) }

    /// Returns the arc cosine (result in radians).
    public func acos() -> Float64 { Float64(raw: libm_acos(self.raw)) }

    /// Returns the arc tangent (result in radians).
    public func atan() -> Float64 { Float64(raw: libm_atan(self.raw)) }

    /// Returns the arc tangent of self/x, with proper quadrant handling.
    public func atan2(x: Float64) -> Float64 { Float64(raw: libm_atan2(self.raw, x.raw)) }

    /// Returns both sine and cosine as a tuple.
    public func sinCos() -> (Float64, Float64) {
        (self.sin(), self.cos())
    }

    // ========================================================================
    // HYPERBOLIC FUNCTIONS
    // ========================================================================

    /// Returns the hyperbolic sine.
    public func sinh() -> Float64 { Float64(raw: libm_sinh(self.raw)) }

    /// Returns the hyperbolic cosine.
    public func cosh() -> Float64 { Float64(raw: libm_cosh(self.raw)) }

    /// Returns the hyperbolic tangent.
    public func tanh() -> Float64 { Float64(raw: libm_tanh(self.raw)) }
    /// Returns the inverse hyperbolic sine.
    public func asinh() -> Float64 { Float64(raw: libm_asinh(self.raw)) }

    /// Returns the inverse hyperbolic cosine.
    public func acosh() -> Float64 { Float64(raw: libm_acosh(self.raw)) }

    /// Returns the inverse hyperbolic tangent.
    public func atanh() -> Float64 { Float64(raw: libm_atanh(self.raw)) }

    // ========================================================================
    // IEEE 754 OPERATIONS
    // ========================================================================

    /// Fused multiply-add: self * a + b with a single rounding.
    /// More accurate than separate multiply and add operations.
    public func fma(a: Float64, b: Float64) -> Float64 {
        Float64(raw: lang.f64_fma(self.raw, a.raw, b.raw))
    }

    /// Returns the magnitude of self with the sign of other.
    public func copysign(from other: Float64) -> Float64 {
        Float64(raw: lang.f64_copysign(self.raw, other.raw))
    }

    /// Returns the smallest representable value greater than self.
    public func nextUp() -> Float64 {
        Float64(raw: libm_nextafter(self.raw, lang.f64_infinity()))
    }

    /// Returns the largest representable value less than self.
    public func nextDown() -> Float64 {
        Float64(raw: libm_nextafter(self.raw, lang.f64_neg(lang.f64_infinity())))
    }

    /// Returns the IEEE remainder of self divided by other.
    public func remainder(dividingBy other: Float64) -> Float64 {
        Float64(raw: libm_remainder(self.raw, other.raw))
    }

    // ========================================================================
    // CLAMPING AND INTERPOLATION
    // ========================================================================

    /// Clamps this value to the given range.
    /// Returns NaN if self is NaN.
    public func clamp(min: Float64, max: Float64) -> Float64 {
        if self.isNaN { self }
        else if self < min { min }
        else if self > max { max }
        else { self }
    }

    /// Linear interpolation from self to other.
    /// Returns self + (other - self) * t.
    public func lerp(to other: Float64, t: Float64) -> Float64 {
        self.add(other.subtract(self).multiply(t))
    }

    // ========================================================================
    // CONVERSION
    // ========================================================================

    /// Converts to Int64, returning None if out of range or not finite.
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

    public func toFloat32() -> Float32 {
        Float32(raw: lang.cast_f64_f32(self.raw))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

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
            var i = digits.byteCount - 1;
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
