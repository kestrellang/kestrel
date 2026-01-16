// UInt32 - 32-bit unsigned integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)
import std.ops.(
    Addable, Subtractable, Multipliable, Divisible, Modulo,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral
)

public struct UInt32:
    UnsignedInteger,
    Integer,
    Comparable,
    Equatable,
    Numeric,
    Hashable,
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
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt64],
    Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32],
    Convertible[Int64]
{
    private var value: lang.i32

    public var raw: lang.i32 { self.value }

    public static var zero: UInt32 { UInt32(intLiteral: 0) }
    public static var one: UInt32 { UInt32(intLiteral: 1) }
    public static var minValue: UInt32 { UInt32(intLiteral: 0) }
    public static var maxValue: UInt32 { UInt32(intLiteral: 4294967295) }
    public static var bitWidth: Int { 32 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i32(value)
    }

    init(raw value: lang.i32) {
        self.value = value
    }

    // Conversions from other integer types
    public init(from other: UInt8) { self.value = lang.cast_i8_i32(other.raw) }
    public init(from other: UInt16) { self.value = lang.cast_i16_i32(other.raw) }
    public init(from other: UInt64) { self.value = lang.cast_i64_i32(other.raw) }
    public init(from other: Int8) { self.value = lang.cast_i8_i32(other.raw) }
    public init(from other: Int16) { self.value = lang.cast_i16_i32(other.raw) }
    public init(from other: Int32) { self.value = other.raw }
    public init(from other: Int64) { self.value = lang.cast_i64_i32(other.raw) }

    public func equals(other: UInt32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.value, other.value))
    }

    public func compare(other: UInt32) -> Ordering {
        if Bool(boolLiteral: lang.i32_unsigned_lt(self.value, other.value)) { .Less }
        else if Bool(boolLiteral: lang.i32_unsigned_gt(self.value, other.value)) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    // Associated type bindings (qualified to avoid ambiguity across protocols)
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

    public func add(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_add(self.value, other.value)) }
    public func subtract(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_sub(self.value, other.value)) }
    public func multiply(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_mul(self.value, other.value)) }
    public func divide(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_div(self.value, other.value)) }
    public func modulo(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt32 { UInt32(raw: lang.i32_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> UInt32 { UInt32(raw: lang.i32_shl(self.value, lang.cast_i64_i32(count))) }
    public func shiftRight(by count: lang.i64) -> UInt32 { UInt32(raw: lang.i32_unsigned_shr(self.value, lang.cast_i64_i32(count))) }
}
