// Int32 - 32-bit signed integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)

public struct Int32:
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
    Convertible[Int16],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    private var value: lang.i32

    public var raw: lang.i32 { self.value }

    public static var zero: Int32 { Int32(intLiteral: 0) }
    public static var one: Int32 { Int32(intLiteral: 1) }
    public static var minValue: Int32 { Int32(intLiteral: -2147483648) }
    public static var maxValue: Int32 { Int32(intLiteral: 2147483647) }
    // public static var bitWidth: Int { 32 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i32(value)
    }

    init(raw value: lang.i32) {
        self.value = value
    }

    public init(from other: Int8) { self.value = lang.cast_i8_i32(other.raw) }
    public init(from other: Int16) { self.value = lang.cast_i16_i32(other.raw) }
    public init(from other: Int64) { self.value = lang.cast_i64_i32(other.raw) }
    public init(from other: UInt8) { self.value = lang.cast_i8_i32(other.raw) }
    public init(from other: UInt16) { self.value = lang.cast_i16_i32(other.raw) }
    public init(from other: UInt32) { self.value = other.raw }
    public init(from other: UInt64) { self.value = lang.cast_i64_i32(other.raw) }

    public func equals(other: Int32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.value, other.value))
    }

    public func compare(other: Int32) -> Ordering {
        if Bool(boolLiteral: lang.i32_signed_lt(self.value, other.value)) { .Less }
        else if Bool(boolLiteral: lang.i32_signed_gt(self.value, other.value)) { .Greater }
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

    public func add(other: Int32) -> Int32 { Int32(raw: lang.i32_add(self.value, other.value)) }
    public func subtract(other: Int32) -> Int32 { Int32(raw: lang.i32_sub(self.value, other.value)) }
    public func multiply(other: Int32) -> Int32 { Int32(raw: lang.i32_mul(self.value, other.value)) }
    public func divide(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_div(self.value, other.value)) }
    public func modulo(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_rem(self.value, other.value)) }
    public func negate() -> Int32 { Int32(raw: lang.i32_neg(self.value)) }
    public func abs() -> Int32 { if Bool(boolLiteral: lang.i32_signed_lt(self.value, 0)) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int32) -> Int32 { Int32(raw: lang.i32_and(self.value, other.value)) }
    public func bitwiseOr(other: Int32) -> Int32 { Int32(raw: lang.i32_or(self.value, other.value)) }
    public func bitwiseXor(other: Int32) -> Int32 { Int32(raw: lang.i32_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int32 { Int32(raw: lang.i32_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> Int32 { Int32(raw: lang.i32_shl(self.value, lang.cast_i64_i32(count))) }
    public func shiftRight(by count: lang.i64) -> Int32 { Int32(raw: lang.i32_signed_shr(self.value, lang.cast_i64_i32(count))) }
}

