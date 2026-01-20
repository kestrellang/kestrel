// UInt32 - 32-bit unsigned integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)

public struct UInt32:
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
    Convertible[UInt16],
    Convertible[UInt64]
{
    public var raw: lang.i32

    public static var zero: UInt32 { UInt32(intLiteral: 0) }
    public static var one: UInt32 { UInt32(intLiteral: 1) }
    public static var minValue: UInt32 { UInt32(intLiteral: lang.i64_neg(0)) }
    public static var maxValue: UInt32 { UInt32(intLiteral: 4294967295) }
    // public static var bitWidth: Int { 32 }

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i32(value)
    }

    init(raw value: lang.i32) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: Int16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: Int32) { self.raw = other.raw }
    public init(from other: Int64) { self.raw = lang.cast_i64_i32(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i32(other.raw) }

    public func equals(other: UInt32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    public func matches(other: UInt32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    public func compare(other: UInt32) -> Ordering {
        if Bool(boolLiteral: lang.i32_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i32_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    public func successor() -> UInt32 { self.add(UInt32.one) }
    public func predecessor() -> UInt32 { self.subtract(UInt32.one) }

    // Associated type bindings
    type Addable.Output = UInt32
    type Subtractable.Output = UInt32
    type Multipliable.Output = UInt32
    type Divisible.Output = UInt32
    type Modulo.Output = UInt32
    
    type BitwiseAnd.Output = UInt32
    type BitwiseOr.Output = UInt32
    type BitwiseXor.Output = UInt32
    type BitwiseNot.Output = UInt32
    type LeftShift.Output = UInt32
    type RightShift.Output = UInt32

    public func add(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_add(self.raw, other.raw)) }
    public func subtract(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_sub(self.raw, other.raw)) }
    public func multiply(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_mul(self.raw, other.raw)) }
    public func divide(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_div(self.raw, other.raw)) }
    public func modulo(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_rem(self.raw, other.raw)) }
    
    
    public func bitwiseAnd(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_and(self.raw, other.raw)) }
    public func bitwiseOr(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_or(self.raw, other.raw)) }
    public func bitwiseXor(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> UInt32 { UInt32(raw: lang.i32_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> UInt32 { UInt32(raw: lang.i32_shl(self.raw, lang.cast_i64_i32(count))) }
    public func shiftRight(by count: lang.i64) -> UInt32 { UInt32(raw: lang.i32_unsigned_shr(self.raw, lang.cast_i64_i32(count))) }
}

