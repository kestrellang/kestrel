// Float32 - 32-bit floating point
// Generated from float.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Formattable,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral, Convertible, Defaultable
)
import std.text.(String)
import std.num.(Int64, Float64)

/// A 32-bit IEEE 754 floating-point type.
///
/// Float32 supports arithmetic, comparison, mathematical functions, and formatting.
///
/// Special values:
/// - `Float32.nan`: Not-a-Number (result of undefined operations like 0/0)
/// - `Float32.infinity`: Positive infinity (result of overflow or 1/0)
///
/// NaN behavior:
/// - NaN is not equal to anything, including itself: `nan == nan` is false
/// - NaN comparisons always return false: `nan < 0`, `nan > 0` are both false
/// - Any arithmetic with NaN produces NaN
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
    public var raw: lang.f32

    // ========================================================================
    // CONSTANTS - Basic Values
    // ========================================================================

    /// The zero value (0.0).
    public static var zero: Float32 { Float32(floatLiteral: 0.0) }

    /// The one value (1.0).
    public static var one: Float32 { Float32(floatLiteral: 1.0) }

    /// The minimum finite value (most negative).
    public static var minValue: Float32 { Float32(floatLiteral: 3.4028235e38).negate() }

    /// The maximum finite value (most positive).
    public static var maxValue: Float32 { Float32(floatLiteral: 3.4028235e38) }

    /// The smallest positive normal value.
    /// Values smaller than this are subnormal (denormalized) and have reduced precision.
    public static var minPositive: Float32 { Float32(floatLiteral: 1.17549435e-38) }

    /// Machine epsilon: the smallest value such that `1.0 + epsilon != 1.0`.
    /// Useful for comparing floating-point values with tolerance.
    public static var epsilon: Float32 { Float32(floatLiteral: 1.1920929e-7) }

    // ========================================================================
    // CONSTANTS - Special Values
    // ========================================================================

    /// Positive infinity. Result of dividing by zero or overflow.
    public static var infinity: Float32 { Float32(raw: lang.f32_infinity()) }

    /// Not-a-Number. Result of undefined operations like 0/0.
    /// NaN is not equal to anything, including itself.
    public static var nan: Float32 { Float32(raw: lang.f32_nan()) }

    // ========================================================================
    // CONSTANTS - Mathematical
    // ========================================================================

    /// Pi (π ≈ 3.14159...). The ratio of a circle's circumference to its diameter.
    public static var pi: Float32 { Float32(floatLiteral: 3.141592653589793) }

    /// Euler's number (e ≈ 2.71828...). Base of the natural logarithm.
    public static var e: Float32 { Float32(floatLiteral: 2.718281828459045) }

    /// Tau (τ = 2π ≈ 6.28318...). The ratio of a circle's circumference to its radius.
    public static var tau: Float32 { Float32(floatLiteral: 6.283185307179586) }

    /// Natural logarithm of 2 (ln(2) ≈ 0.693...).
    public static var ln2: Float32 { Float32(floatLiteral: 0.6931471805599453) }

    /// Natural logarithm of 10 (ln(10) ≈ 2.302...).
    public static var ln10: Float32 { Float32(floatLiteral: 2.302585092994046) }

    /// Square root of 2 (√2 ≈ 1.414...).
    public static var sqrt2: Float32 { Float32(floatLiteral: 1.4142135623730951) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates a Float32 from a floating-point literal.
    public init(floatLiteral value: lang.f64) {
        self.raw = lang.cast_f64_f32(value)
    }

    /// Creates a Float32 with the default value (zero).
    public init() {
        self.init(floatLiteral: 0.0)
    }

    /// Creates a Float32 from an integer literal.
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f32(value)
    }

    /// Creates a Float32 from a raw `lang.f32` value.
    init(raw value: lang.f32) {
        self.raw = value
    }

    /// Creates a Float32 from an Int64.
    public init(from value: Int64) {
        self.raw = lang.cast_i64_f32(value.raw)
    }

    /// Creates a Float32 from a Float64.
    public init(from value: Float64) {
        self.raw = lang.cast_f64_f32(value.raw)
    }

    // ========================================================================
    // CLASSIFICATION (Properties)
    // ========================================================================

    /// Returns true if this value is NaN (Not-a-Number).
    public var isNaN: Bool { get {
        Bool(boolLiteral: lang.f32_is_nan(self.raw))
    }}

    /// Returns true if this value is positive or negative infinity.
    public var isInfinite: Bool { get {
        Bool(boolLiteral: lang.f32_is_infinite(self.raw))
    }}

    /// Returns true if this value is finite (not NaN and not infinite).
    public var isFinite: Bool { get {
        not self.isNaN and not self.isInfinite
    }}

    /// Returns true if this value is a normal number (not zero, subnormal, infinite, or NaN).
    public var isNormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() >= Float32.minPositive
    }}

    /// Returns true if this value is subnormal (denormalized).
    /// Subnormal numbers have reduced precision but allow gradual underflow.
    public var isSubnormal: Bool { get {
        self.isFinite and not self.isZero and self.abs() < Float32.minPositive
    }}

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Returns -1.0 if negative, 0.0 if zero, 1.0 if positive, or NaN if NaN.
    public var sign: Float32 { get {
        if self.isNaN { Float32.nan }
        else if self < 0.0 { Float32(raw: lang.f32_neg(1.0)) }
        else if self > 0.0 { Float32(floatLiteral: 1.0) }
        else { Float32(floatLiteral: 0.0) }
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
    public func equals(other: Float32) -> Bool {
        Bool(boolLiteral: lang.f32_eq(self.raw, other.raw))
    }

    /// Compares this value to another, returning an Ordering.
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

    /// Adds two values.
    public func add(other: Float32) -> Float32 { Float32(raw: lang.f32_add(self.raw, other.raw)) }

    /// Subtracts two values.
    public func subtract(other: Float32) -> Float32 { Float32(raw: lang.f32_sub(self.raw, other.raw)) }

    /// Multiplies two values.
    public func multiply(other: Float32) -> Float32 { Float32(raw: lang.f32_mul(self.raw, other.raw)) }

    /// Divides two values.
    public func divide(other: Float32) -> Float32 { Float32(raw: lang.f32_div(self.raw, other.raw)) }
    /// Negates this value.
    public func negate() -> Float32 { Float32(raw: lang.f32_neg(self.raw)) }

    // ========================================================================
    // BASIC MATHEMATICAL FUNCTIONS
    // ========================================================================

    /// Returns the absolute value.
    public func abs() -> Float32 {
        if Bool(boolLiteral: lang.f32_lt(self.raw, 0.0)) { self.negate() } else { self }
    }

    /// Rounds down to the nearest integer.
    public func floor() -> Float32 { Float32(raw: lang.f32_floor(self.raw)) }

    /// Rounds up to the nearest integer.
    public func ceil() -> Float32 { Float32(raw: lang.f32_ceil(self.raw)) }

    /// Rounds to the nearest integer (half away from zero).
    public func round() -> Float32 { Float32(raw: lang.f32_round(self.raw)) }

    /// Truncates toward zero.
    public func trunc() -> Float32 { Float32(raw: lang.f32_trunc(self.raw)) }

    /// Returns the fractional part (self - trunc(self)).
    public func fract() -> Float32 {
        self.subtract(self.trunc())
    }

    /// Returns the square root.
    public func sqrt() -> Float32 { Float32(raw: lang.f32_sqrt(self.raw)) }

    /// Returns the cube root.
    public func cbrt() -> Float32 {
        Float32(raw: libm_cbrtf(self.raw))
    }

    /// Returns the hypotenuse: sqrt(self² + other²).
    public func hypot(other: Float32) -> Float32 {
        Float32(raw: libm_hypotf(self.raw, other.raw))
    }

    // ========================================================================
    // EXPONENTIAL AND LOGARITHMIC FUNCTIONS
    // ========================================================================

    /// Returns e^self (exponential).
    public func exp() -> Float32 { Float32(raw: libm_expf(self.raw)) }

    /// Returns 2^self.
    public func exp2() -> Float32 { Float32(raw: libm_exp2f(self.raw)) }

    /// Returns e^self - 1, accurate for small values.
    public func expm1() -> Float32 { Float32(raw: libm_expm1f(self.raw)) }

    /// Returns the natural logarithm (base e).
    public func ln() -> Float32 { Float32(raw: libm_logf(self.raw)) }

    /// Returns ln(1 + self), accurate for small values.
    public func ln1p() -> Float32 { Float32(raw: libm_log1pf(self.raw)) }

    /// Returns the base-2 logarithm.
    public func log2() -> Float32 { Float32(raw: libm_log2f(self.raw)) }

    /// Returns the base-10 logarithm.
    public func log10() -> Float32 { Float32(raw: libm_log10f(self.raw)) }

    /// Returns the logarithm with the given base.
    public func log(base: Float32) -> Float32 {
        self.ln().divide(base.ln())
    }

    /// Raises self to the given floating-point power.
    public func pow(exponent: Float32) -> Float32 {
        Float32(raw: libm_powf(self.raw, exponent.raw))
    }

    /// Raises self to the given integer power.
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
    public func sin() -> Float32 { Float32(raw: libm_sinf(self.raw)) }

    /// Returns the cosine (argument in radians).
    public func cos() -> Float32 { Float32(raw: libm_cosf(self.raw)) }

    /// Returns the tangent (argument in radians).
    public func tan() -> Float32 { Float32(raw: libm_tanf(self.raw)) }

    /// Returns the arc sine (result in radians).
    public func asin() -> Float32 { Float32(raw: libm_asinf(self.raw)) }

    /// Returns the arc cosine (result in radians).
    public func acos() -> Float32 { Float32(raw: libm_acosf(self.raw)) }

    /// Returns the arc tangent (result in radians).
    public func atan() -> Float32 { Float32(raw: libm_atanf(self.raw)) }

    /// Returns the arc tangent of self/x, with proper quadrant handling.
    public func atan2(x: Float32) -> Float32 { Float32(raw: libm_atan2f(self.raw, x.raw)) }

    /// Returns both sine and cosine as a tuple.
    public func sinCos() -> (Float32, Float32) {
        (self.sin(), self.cos())
    }

    // ========================================================================
    // HYPERBOLIC FUNCTIONS
    // ========================================================================

    /// Returns the hyperbolic sine.
    public func sinh() -> Float32 { Float32(raw: libm_sinhf(self.raw)) }

    /// Returns the hyperbolic cosine.
    public func cosh() -> Float32 { Float32(raw: libm_coshf(self.raw)) }

    /// Returns the hyperbolic tangent.
    public func tanh() -> Float32 { Float32(raw: libm_tanhf(self.raw)) }
    /// Returns the inverse hyperbolic sine.
    public func asinh() -> Float32 { Float32(raw: libm_asinhf(self.raw)) }

    /// Returns the inverse hyperbolic cosine.
    public func acosh() -> Float32 { Float32(raw: libm_acoshf(self.raw)) }

    /// Returns the inverse hyperbolic tangent.
    public func atanh() -> Float32 { Float32(raw: libm_atanhf(self.raw)) }

    // ========================================================================
    // IEEE 754 OPERATIONS
    // ========================================================================

    /// Fused multiply-add: self * a + b with a single rounding.
    /// More accurate than separate multiply and add operations.
    public func fma(a: Float32, b: Float32) -> Float32 {
        Float32(raw: lang.f32_fma(self.raw, a.raw, b.raw))
    }

    /// Returns the magnitude of self with the sign of other.
    public func copysign(from other: Float32) -> Float32 {
        Float32(raw: lang.f32_copysign(self.raw, other.raw))
    }

    /// Returns the smallest representable value greater than self.
    public func nextUp() -> Float32 {
        Float32(raw: libm_nextafterf(self.raw, lang.f32_infinity()))
    }

    /// Returns the largest representable value less than self.
    public func nextDown() -> Float32 {
        Float32(raw: libm_nextafterf(self.raw, lang.f32_neg(lang.f32_infinity())))
    }

    /// Returns the IEEE remainder of self divided by other.
    public func remainder(dividingBy other: Float32) -> Float32 {
        Float32(raw: libm_remainderf(self.raw, other.raw))
    }

    // ========================================================================
    // CLAMPING AND INTERPOLATION
    // ========================================================================

    /// Clamps this value to the given range.
    /// Returns NaN if self is NaN.
    public func clamp(min: Float32, max: Float32) -> Float32 {
        if self.isNaN { self }
        else if self < min { min }
        else if self > max { max }
        else { self }
    }

    /// Linear interpolation from self to other.
    /// Returns self + (other - self) * t.
    public func lerp(to other: Float32, t: Float32) -> Float32 {
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
        .Some(Int64(raw: lang.cast_f32_i64(truncated.raw)))
    }

    public func toFloat64() -> Float64 {
        Float64(raw: lang.cast_f32_f64(self.raw))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    public static func parse(string: String) -> Float32? {
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
                return .Some(Float32.nan)
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
                return .Some(Float32.infinity)
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
                return .Some(Float32(raw: lang.f32_neg(lang.f32_infinity())))
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
                return .Some(Float32.infinity)
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
                return .Some(Float32.infinity)
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
        var integerPart: Float32 = 0.0;
        var hasIntegerPart = false;
        var currentByte: Int64 = Int64(from: string.byteAtUnchecked(index));

        while index < len and currentByte >= 48 and currentByte <= 57 {
            let digit = Float32(from: currentByte - 48);
            integerPart = integerPart * 10.0 + digit;
            hasIntegerPart = true;
            index = index + 1;
            if index < len {
                currentByte = Int64(from: string.byteAtUnchecked(index))
            }
        }

        // Parse fractional part
        var fractionalPart: Float32 = 0.0;
        var hasFractionalPart = false;

        if index < len and currentByte == 46 {  // '.'
            index = index + 1;
            var divisor: Float32 = 10.0;

            if index < len {
                currentByte = Int64(from: string.byteAtUnchecked(index));
                while index < len and currentByte >= 48 and currentByte <= 57 {
                    let digit = Float32(from: currentByte - 48);
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

