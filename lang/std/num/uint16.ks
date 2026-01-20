// UInt16 - 16-bit unsigned integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)

public struct UInt16:
    UnsignedInteger,
    Steppable,
    Comparable,
    Equatable,
    Matchable,
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
    Convertible[UInt8],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i16

    public static var zero: UInt16 { UInt16(intLiteral: 0) }
    public static var one: UInt16 { UInt16(intLiteral: 1) }
    public static var minValue: UInt16 { UInt16(intLiteral: lang.i64_neg(0)) }
    public static var maxValue: UInt16 { UInt16(intLiteral: 65535) }
    // public static var bitWidth: Int { 16 }

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i16(value)
    }

    init(raw value: lang.i16) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i16(other.raw) }
    public init(from other: Int16) { self.raw = other.raw }
    public init(from other: Int32) { self.raw = lang.cast_i32_i16(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i16(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i16(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i16(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i16(other.raw) }

    public func equals(other: UInt16) -> Bool {
        Bool(boolLiteral: lang.i16_eq(self.raw, other.raw))
    }

    public func matches(other: UInt16) -> Bool {
        Bool(boolLiteral: lang.i16_eq(self.raw, other.raw))
    }

    public func compare(other: UInt16) -> Ordering {
        if Bool(boolLiteral: lang.i16_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i16_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    public func successor() -> UInt16 { self.add(UInt16.one) }
    public func predecessor() -> UInt16 { self.subtract(UInt16.one) }

    // Associated type bindings
    type Addable.Output = UInt16
    type Subtractable.Output = UInt16
    type Multipliable.Output = UInt16
    type Divisible.Output = UInt16
    type Modulo.Output = UInt16
    
    type BitwiseAnd.Output = UInt16
    type BitwiseOr.Output = UInt16
    type BitwiseXor.Output = UInt16
    type BitwiseNot.Output = UInt16
    type LeftShift.Output = UInt16
    type RightShift.Output = UInt16

    public func add(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_add(self.raw, other.raw)) }
    public func subtract(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_sub(self.raw, other.raw)) }
    public func multiply(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_mul(self.raw, other.raw)) }
    public func divide(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_unsigned_div(self.raw, other.raw)) }
    public func modulo(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_unsigned_rem(self.raw, other.raw)) }
    
    
    public func bitwiseAnd(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_and(self.raw, other.raw)) }
    public func bitwiseOr(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_or(self.raw, other.raw)) }
    public func bitwiseXor(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> UInt16 { UInt16(raw: lang.i16_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> UInt16 { UInt16(raw: lang.i16_shl(self.raw, lang.cast_i64_i16(count))) }
    public func shiftRight(by count: lang.i64) -> UInt16 { UInt16(raw: lang.i16_unsigned_shr(self.raw, lang.cast_i64_i16(count))) }
}

