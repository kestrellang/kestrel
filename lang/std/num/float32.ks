// Float32 - 32-bit floating point
// Generated from float.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Formattable,
    Addable, Subtractable, Multipliable, Divisible, Negatable,
    ExpressibleByFloatLiteral, ExpressibleByIntLiteral
)
import std.text.(String)

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
    FFISafe
{
    public var raw: lang.f32

    public static var zero: Float32 { Float32(floatLiteral: 0.0) }
    public static var one: Float32 { Float32(floatLiteral: 1.0) }
    public static var infinity: Float32 { Float32(raw: lang.f32_infinity()) }
    public static var nan: Float32 { Float32(raw: lang.f32_nan()) }
    // public static var bitWidth: Int { 32 }

    // Mathematical constants
    public static var pi: Float32 { Float32(floatLiteral: 3.141592653589793) }
    public static var e: Float32 { Float32(floatLiteral: 2.718281828459045) }
    public static var tau: Float32 { Float32(floatLiteral: 6.283185307179586) }

    public init(floatLiteral value: lang.f64) {
        self.raw = lang.cast_f64_f32(value)
    }

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_f32(value)
    }

    init(raw value: lang.f32) {
        self.raw = value
    }

    public func isNaN() -> Bool {
        Bool(boolLiteral: lang.f32_is_nan(self.raw))
    }

    public func isInfinite() -> Bool {
        Bool(boolLiteral: lang.f32_is_infinite(self.raw))
    }

    public func isFinite() -> Bool {
        not self.isNaN() and not self.isInfinite()
    }

    public func equals(other: Float32) -> Bool {
        Bool(boolLiteral: lang.f32_eq(self.raw, other.raw))
    }

    public func compare(other: Float32) -> Ordering {
        if Bool(boolLiteral: lang.f32_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.f32_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // Associated type bindings
    type Addable.Output = Float32
    type Subtractable.Output = Float32
    type Multipliable.Output = Float32
    type Divisible.Output = Float32
    type Negatable.Output = Float32

    public func add(other: Float32) -> Float32 { Float32(raw: lang.f32_add(self.raw, other.raw)) }
    public func subtract(other: Float32) -> Float32 { Float32(raw: lang.f32_sub(self.raw, other.raw)) }
    public func multiply(other: Float32) -> Float32 { Float32(raw: lang.f32_mul(self.raw, other.raw)) }
    public func divide(other: Float32) -> Float32 { Float32(raw: lang.f32_div(self.raw, other.raw)) }
    public func negate() -> Float32 { Float32(raw: lang.f32_neg(self.raw)) }

    public func abs() -> Float32 {
        if Bool(boolLiteral: lang.f32_lt(self.raw, 0.0)) { self.negate() } else { self }
    }

    public func floor() -> Float32 { Float32(raw: lang.f32_floor(self.raw)) }
    public func ceil() -> Float32 { Float32(raw: lang.f32_ceil(self.raw)) }
    public func round() -> Float32 { Float32(raw: lang.f32_round(self.raw)) }
    public func trunc() -> Float32 { Float32(raw: lang.f32_trunc(self.raw)) }
    public func sqrt() -> Float32 { Float32(raw: lang.f32_sqrt(self.raw)) }

    // Formattable
    public func format() -> String {
        // Handle special cases
        if self.isNaN() {
            return "NaN"
        }
        if self.isInfinite() {
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
                digits.appendByte(UInt8(from: digit + 48));
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
            result.appendByte(UInt8(from: digit + 48));
            fracPart = fracPart - Float32(raw: lang.cast_i64_f32(digit.raw));
            digitCount = digitCount + 1
        }

        result
    }}

