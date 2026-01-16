// Int16 - 16-bit signed integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)

public struct Int16:
    SignedInteger,
    Steppable,
    Comparable,
    Equatable,
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
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    private var value: lang.i16

    public var raw: lang.i16 { self.value }

    public static var zero: Int16 { Int16(intLiteral: 0) }
    public static var one: Int16 { Int16(intLiteral: 1) }
    public static var minValue: Int16 { Int16(intLiteral: -32768) }
    public static var maxValue: Int16 { Int16(intLiteral: 32767) }
    // public static var bitWidth: Int { 16 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i16(value)
    }

    init(raw value: lang.i16) {
        self.value = value
    }

    public init(from other: Int8) { self.value = lang.cast_i8_i16(other.raw) }
    public init(from other: Int32) { self.value = lang.cast_i32_i16(other.raw) }
    public init(from other: Int64) { self.value = lang.cast_i64_i16(other.raw) }
    public init(from other: UInt8) { self.value = lang.cast_i8_i16(other.raw) }
    public init(from other: UInt16) { self.value = other.raw }
    public init(from other: UInt32) { self.value = lang.cast_i32_i16(other.raw) }
    public init(from other: UInt64) { self.value = lang.cast_i64_i16(other.raw) }

    public func equals(other: Int16) -> Bool {
        Bool(boolLiteral: lang.i16_eq(self.value, other.value))
    }

    public func compare(other: Int16) -> Ordering {
        if Bool(boolLiteral: lang.i16_signed_lt(self.value, other.value)) { .Less }
        else if Bool(boolLiteral: lang.i16_signed_gt(self.value, other.value)) { .Greater }
        else { .Equal }
    }

    public func successor() -> Int16 { self.add(Int16.one) }
    public func predecessor() -> Int16 { self.subtract(Int16.one) }

    // Associated type bindings
    type Addable.Output = Int16
    type Subtractable.Output = Int16
    type Multipliable.Output = Int16
    type Divisible.Output = Int16
    type Modulo.Output = Int16
    type Negatable.Output = Int16
    type BitwiseAnd.Output = Int16
    type BitwiseOr.Output = Int16
    type BitwiseXor.Output = Int16
    type BitwiseNot.Output = Int16
    type LeftShift.Output = Int16
    type RightShift.Output = Int16

    public func add(other: Int16) -> Int16 { Int16(raw: lang.i16_add(self.value, other.value)) }
    public func subtract(other: Int16) -> Int16 { Int16(raw: lang.i16_sub(self.value, other.value)) }
    public func multiply(other: Int16) -> Int16 { Int16(raw: lang.i16_mul(self.value, other.value)) }
    public func divide(other: Int16) -> Int16 { Int16(raw: lang.i16_signed_div(self.value, other.value)) }
    public func modulo(other: Int16) -> Int16 { Int16(raw: lang.i16_signed_rem(self.value, other.value)) }
    public func negate() -> Int16 { Int16(raw: lang.i16_neg(self.value)) }
    public func abs() -> Int16 { if Bool(boolLiteral: lang.i16_signed_lt(self.value, 0)) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int16) -> Int16 { Int16(raw: lang.i16_and(self.value, other.value)) }
    public func bitwiseOr(other: Int16) -> Int16 { Int16(raw: lang.i16_or(self.value, other.value)) }
    public func bitwiseXor(other: Int16) -> Int16 { Int16(raw: lang.i16_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int16 { Int16(raw: lang.i16_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> Int16 { Int16(raw: lang.i16_shl(self.value, lang.cast_i64_i16(count))) }
    public func shiftRight(by count: lang.i64) -> Int16 { Int16(raw: lang.i16_signed_shr(self.value, lang.cast_i64_i16(count))) }
}

