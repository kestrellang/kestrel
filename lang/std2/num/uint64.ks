// UInt64 - 64-bit unsigned integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)

public struct UInt64:
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
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32]
{
    private var value: lang.i64

    public var raw: lang.i64 { self.value }

    public static var zero: UInt64 { UInt64(intLiteral: 0) }
    public static var one: UInt64 { UInt64(intLiteral: 1) }
    public static var minValue: UInt64 { UInt64(intLiteral: 0) }
    public static var maxValue: UInt64 { UInt64(intLiteral: 18446744073709551615) }
    // public static var bitWidth: Int { 64 }

    public init(intLiteral value: lang.i64) {
        self.value = value
    }

    init(raw value: lang.i64) {
        self.value = value
    }

    public init(from other: Int8) { self.value = lang.cast_i8_i64(other.raw) }
    public init(from other: Int16) { self.value = lang.cast_i16_i64(other.raw) }
    public init(from other: Int32) { self.value = lang.cast_i32_i64(other.raw) }
    public init(from other: Int64) { self.value = other.raw }
    public init(from other: UInt8) { self.value = lang.cast_i8_i64(other.raw) }
    public init(from other: UInt16) { self.value = lang.cast_i16_i64(other.raw) }
    public init(from other: UInt32) { self.value = lang.cast_i32_i64(other.raw) }

    public func equals(other: UInt64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.value, other.value))
    }

    public func compare(other: UInt64) -> Ordering {
        if Bool(boolLiteral: lang.i64_unsigned_lt(self.value, other.value)) { .Less }
        else if Bool(boolLiteral: lang.i64_unsigned_gt(self.value, other.value)) { .Greater }
        else { .Equal }
    }

    public func successor() -> UInt64 { self.add(UInt64.one) }
    public func predecessor() -> UInt64 { self.subtract(UInt64.one) }

    // Associated type bindings
    type Addable.Output = UInt64
    type Subtractable.Output = UInt64
    type Multipliable.Output = UInt64
    type Divisible.Output = UInt64
    type Modulo.Output = UInt64
    
    type BitwiseAnd.Output = UInt64
    type BitwiseOr.Output = UInt64
    type BitwiseXor.Output = UInt64
    type BitwiseNot.Output = UInt64
    type LeftShift.Output = UInt64
    type RightShift.Output = UInt64

    public func add(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_add(self.value, other.value)) }
    public func subtract(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_sub(self.value, other.value)) }
    public func multiply(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_mul(self.value, other.value)) }
    public func divide(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_div(self.value, other.value)) }
    public func modulo(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_rem(self.value, other.value)) }
    
    
    public func bitwiseAnd(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt64 { UInt64(raw: lang.i64_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_shl(self.value, count)) }
    public func shiftRight(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_unsigned_shr(self.value, count)) }
}

// UInt - platform-sized unsigned integer (alias to UInt64 on 64-bit platforms)
public type UInt = UInt64
