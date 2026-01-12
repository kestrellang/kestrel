// UInt16 - 16-bit unsigned integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)
import std.ops.(
    Addable, Subtractable, Multipliable, Divisible, Modulo,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral
)

public struct UInt16:
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
    FFISafe
{
    private var value: lang.i16

    public static var zero: UInt16 { UInt16(intLiteral: 0) }
    public static var one: UInt16 { UInt16(intLiteral: 1) }
    public static var minValue: UInt16 { UInt16(intLiteral: 0) }
    public static var maxValue: UInt16 { UInt16(intLiteral: 65535) }
    public static var bitWidth: Int { 16 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i16(value)
    }

    init(raw value: lang.i16) {
        self.value = value
    }

    public func equals(other: UInt16) -> Bool {
        lang.i16_eq(self.value, other.value)
    }

    public func compare(other: UInt16) -> Ordering {
        if lang.i16_unsigned_lt(self.value, other.value) { .Less }
        else if lang.i16_unsigned_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = UInt16

    public func add(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_add(self.value, other.value)) }
    public func subtract(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_sub(self.value, other.value)) }
    public func multiply(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_mul(self.value, other.value)) }
    public func divide(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_unsigned_div(self.value, other.value)) }
    public func mod(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_unsigned_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt16 { UInt16(raw: lang.i16_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> UInt16 { UInt16(raw: lang.i16_shl(self.value, lang.cast_i64_i16(count))) }
    public func shiftRight(by count: lang.i64) -> UInt16 { UInt16(raw: lang.i16_unsigned_shr(self.value, lang.cast_i64_i16(count))) }
}
