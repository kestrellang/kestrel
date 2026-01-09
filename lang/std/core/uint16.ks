// UInt16 - 16-bit unsigned integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)

public struct UInt16:
    UnsignedInteger,
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
    private var value: lang.u16

    public static var zero: UInt16 { UInt16(value: 0) }
    public static var one: UInt16 { UInt16(value: 1) }
    public static var minValue: UInt16 { UInt16(value: 0) }
    public static var maxValue: UInt16 { UInt16(value: 65535) }
    public static var bitWidth: Int { 16 }

    public init(intLiteral value: Int) {
        self.value = lang.cast_i64_u16(value)
    }

    public func equals(other: UInt16) -> Bool {
        lang.u16_eq(self.value, other.value)
    }

    public func compare(other: UInt16) -> Ordering {
        if lang.u16_lt(self.value, other.value) { .Less }
        else if lang.u16_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](into hasher: mutating H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = UInt16

    public func add(other: UInt16) -> UInt16 { UInt16(value: lang.u16_add(self.value, other.value)) }
    public func subtract(other: UInt16) -> UInt16 { UInt16(value: lang.u16_sub(self.value, other.value)) }
    public func multiply(other: UInt16) -> UInt16 { UInt16(value: lang.u16_mul(self.value, other.value)) }
    public func divide(other: UInt16) -> UInt16 { UInt16(value: lang.u16_div(self.value, other.value)) }
    public func mod(other: UInt16) -> UInt16 { UInt16(value: lang.u16_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt16) -> UInt16 { UInt16(value: lang.u16_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt16) -> UInt16 { UInt16(value: lang.u16_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt16) -> UInt16 { UInt16(value: lang.u16_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt16 { UInt16(value: lang.u16_not(self.value)) }
    public func shiftLeft(by count: Int) -> UInt16 { UInt16(value: lang.u16_shl(self.value, lang.cast_i64_u16(count))) }
    public func shiftRight(by count: Int) -> UInt16 { UInt16(value: lang.u16_shr(self.value, lang.cast_i64_u16(count))) }
}
