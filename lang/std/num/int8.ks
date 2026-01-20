// Int8 - 8-bit signed integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable, Formattable,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)
import std.text.(String)

public struct Int8:
    SignedInteger,
    Steppable,
    Comparable,
    Equatable,
    Matchable,
    Formattable,
    Addable,
    Subtractable,
    Multipliable,
    Divisible,
    Modulo,
    Negatable,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseNot,
    LeftShift,
    RightShift,
    ExpressibleByIntLiteral,
    FFISafe,
        Convertible[Int16],
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i8

    public static var zero: Int8 { Int8(intLiteral: 0) }
    public static var one: Int8 { Int8(intLiteral: 1) }
    public static var minValue: Int8 { Int8(intLiteral: lang.i64_neg(128)) }
    public static var maxValue: Int8 { Int8(intLiteral: 127) }
    // public static var bitWidth: Int { 8 }

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i8(value)
    }

    init(raw value: lang.i8) {
        self.raw = value
    }

    public init(from other: Int16) { self.raw = lang.cast_i16_i8(other.raw) }
    public init(from other: Int32) { self.raw = lang.cast_i32_i8(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i8(other.raw) }
    public init(from other: UInt8) { self.raw = other.raw }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i8(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i8(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i8(other.raw) }

    public func equals(other: Int8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.raw, other.raw))
    }

    public func matches(other: Int8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.raw, other.raw))
    }

    public func compare(other: Int8) -> Ordering {
        if Bool(boolLiteral: lang.i8_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i8_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    public func successor() -> Int8 { self.add(Int8.one) }
    public func predecessor() -> Int8 { self.subtract(Int8.one) }

    // Associated type bindings
    type Addable.Output = Int8
    type Subtractable.Output = Int8
    type Multipliable.Output = Int8
    type Divisible.Output = Int8
    type Modulo.Output = Int8
    type Negatable.Output = Int8
    type BitwiseAnd.Output = Int8
    type BitwiseOr.Output = Int8
    type BitwiseXor.Output = Int8
    type BitwiseNot.Output = Int8
    type LeftShift.Output = Int8
    type RightShift.Output = Int8

    public func add(other: Int8) -> Int8 { Int8(raw: lang.i8_add(self.raw, other.raw)) }
    public func subtract(other: Int8) -> Int8 { Int8(raw: lang.i8_sub(self.raw, other.raw)) }
    public func multiply(other: Int8) -> Int8 { Int8(raw: lang.i8_mul(self.raw, other.raw)) }
    public func divide(other: Int8) -> Int8 { Int8(raw: lang.i8_signed_div(self.raw, other.raw)) }
    public func modulo(other: Int8) -> Int8 { Int8(raw: lang.i8_signed_rem(self.raw, other.raw)) }
    public func negate() -> Int8 { Int8(raw: lang.i8_neg(self.raw)) }
    public func abs() -> Int8 { if Bool(boolLiteral: lang.i8_signed_lt(self.raw, 0)) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int8) -> Int8 { Int8(raw: lang.i8_and(self.raw, other.raw)) }
    public func bitwiseOr(other: Int8) -> Int8 { Int8(raw: lang.i8_or(self.raw, other.raw)) }
    public func bitwiseXor(other: Int8) -> Int8 { Int8(raw: lang.i8_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> Int8 { Int8(raw: lang.i8_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> Int8 { Int8(raw: lang.i8_shl(self.raw, lang.cast_i64_i8(count))) }
    public func shiftRight(by count: lang.i64) -> Int8 { Int8(raw: lang.i8_signed_shr(self.raw, lang.cast_i64_i8(count))) }

    // Formattable
    public func format() -> String {
        if self == Int8.zero {
            return "0"
        }

        var result = String();
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        let ten: Int8 = 10;
        while n != Int8.zero {
            let digit: Int8 = n % ten;
            let charCode: Int64 = Int64(from: digit) + 48;
            result.appendByte(UInt8(from: charCode));
            n = n / ten
        }

        if isNegative {
            result.appendByte(45)  // '-'
        }

        // Reverse the string
        var reversed = String();
        var i = result.byteCount() - 1;
        while i >= 0 {
            reversed.appendByte(result.byteAtUnchecked(i));
            i = i - 1
        }
        reversed
    }}

