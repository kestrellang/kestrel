// UInt8 - 8-bit unsigned integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)

public struct UInt8:
    UnsignedInteger,
    Steppable,
    Comparable,
    Equatable,
    Addable,
    Subtractable,
    Multipliable,
    Divisible,
    Modulo,
    
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
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    private var value: lang.i8

    public var raw: lang.i8 { self.value }

    public static var zero: UInt8 { UInt8(intLiteral: 0) }
    public static var one: UInt8 { UInt8(intLiteral: 1) }
    public static var minValue: UInt8 { UInt8(intLiteral: 0) }
    public static var maxValue: UInt8 { UInt8(intLiteral: 255) }
    // public static var bitWidth: Int { 8 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i8(value)
    }

    init(raw value: lang.i8) {
        self.value = value
    }

    public init(from other: Int8) { self.value = other.raw }
    public init(from other: Int16) { self.value = lang.cast_i16_i8(other.raw) }
    public init(from other: Int32) { self.value = lang.cast_i32_i8(other.raw) }
    public init(from other: Int64) { self.value = lang.cast_i64_i8(other.raw) }
    public init(from other: UInt16) { self.value = lang.cast_i16_i8(other.raw) }
    public init(from other: UInt32) { self.value = lang.cast_i32_i8(other.raw) }
    public init(from other: UInt64) { self.value = lang.cast_i64_i8(other.raw) }

    public func equals(other: UInt8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.value, other.value))
    }

    public func compare(other: UInt8) -> Ordering {
        if Bool(boolLiteral: lang.i8_unsigned_lt(self.value, other.value)) { .Less }
        else if Bool(boolLiteral: lang.i8_unsigned_gt(self.value, other.value)) { .Greater }
        else { .Equal }
    }

    public func successor() -> UInt8 { self.add(UInt8.one) }
    public func predecessor() -> UInt8 { self.subtract(UInt8.one) }

    // Associated type bindings
    type Addable.Output = UInt8
    type Subtractable.Output = UInt8
    type Multipliable.Output = UInt8
    type Divisible.Output = UInt8
    type Modulo.Output = UInt8
    
    type BitwiseAnd.Output = UInt8
    type BitwiseOr.Output = UInt8
    type BitwiseXor.Output = UInt8
    type BitwiseNot.Output = UInt8
    type LeftShift.Output = UInt8
    type RightShift.Output = UInt8

    public func add(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_add(self.value, other.value)) }
    public func subtract(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_sub(self.value, other.value)) }
    public func multiply(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_mul(self.value, other.value)) }
    public func divide(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_unsigned_div(self.value, other.value)) }
    public func modulo(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_unsigned_rem(self.value, other.value)) }
    
    
    public func bitwiseAnd(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt8 { UInt8(raw: lang.i8_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> UInt8 { UInt8(raw: lang.i8_shl(self.value, lang.cast_i64_i8(count))) }
    public func shiftRight(by count: lang.i64) -> UInt8 { UInt8(raw: lang.i8_unsigned_shr(self.value, lang.cast_i64_i8(count))) }
}

