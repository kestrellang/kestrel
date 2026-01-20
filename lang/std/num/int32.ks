// Int32 - 32-bit signed integer
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

public struct Int32:
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
        Convertible[Int8],
    Convertible[Int16],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i32

    public static var zero: Int32 { Int32(intLiteral: 0) }
    public static var one: Int32 { Int32(intLiteral: 1) }
    public static var minValue: Int32 { Int32(intLiteral: lang.i64_neg(2147483648)) }
    public static var maxValue: Int32 { Int32(intLiteral: 2147483647) }
    // public static var bitWidth: Int { 32 }

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i32(value)
    }

    init(raw value: lang.i32) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: Int16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i32(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: UInt32) { self.raw = other.raw }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i32(other.raw) }

    public func equals(other: Int32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    public func matches(other: Int32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    public func compare(other: Int32) -> Ordering {
        if Bool(boolLiteral: lang.i32_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i32_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    public func successor() -> Int32 { self.add(Int32.one) }
    public func predecessor() -> Int32 { self.subtract(Int32.one) }

    // Associated type bindings
    type Addable.Output = Int32
    type Subtractable.Output = Int32
    type Multipliable.Output = Int32
    type Divisible.Output = Int32
    type Modulo.Output = Int32
    type Negatable.Output = Int32
    type BitwiseAnd.Output = Int32
    type BitwiseOr.Output = Int32
    type BitwiseXor.Output = Int32
    type BitwiseNot.Output = Int32
    type LeftShift.Output = Int32
    type RightShift.Output = Int32

    public func add(other: Int32) -> Int32 { Int32(raw: lang.i32_add(self.raw, other.raw)) }
    public func subtract(other: Int32) -> Int32 { Int32(raw: lang.i32_sub(self.raw, other.raw)) }
    public func multiply(other: Int32) -> Int32 { Int32(raw: lang.i32_mul(self.raw, other.raw)) }
    public func divide(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_div(self.raw, other.raw)) }
    public func modulo(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_rem(self.raw, other.raw)) }
    public func negate() -> Int32 { Int32(raw: lang.i32_neg(self.raw)) }
    public func abs() -> Int32 { if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0)) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int32) -> Int32 { Int32(raw: lang.i32_and(self.raw, other.raw)) }
    public func bitwiseOr(other: Int32) -> Int32 { Int32(raw: lang.i32_or(self.raw, other.raw)) }
    public func bitwiseXor(other: Int32) -> Int32 { Int32(raw: lang.i32_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> Int32 { Int32(raw: lang.i32_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> Int32 { Int32(raw: lang.i32_shl(self.raw, lang.cast_i64_i32(count))) }
    public func shiftRight(by count: lang.i64) -> Int32 { Int32(raw: lang.i32_signed_shr(self.raw, lang.cast_i64_i32(count))) }

    // Formattable
    public func format() -> String {
        if self == Int32.zero {
            return "0"
        }

        var result = String();
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        let ten: Int32 = 10;
        while n != Int32.zero {
            let digit: Int32 = n % ten;
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

