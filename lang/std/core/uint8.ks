// UInt8 - 8-bit unsigned integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)

public struct UInt8:
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
    private var value: lang.u8

    public static var zero: UInt8 { UInt8(value: 0) }
    public static var one: UInt8 { UInt8(value: 1) }
    public static var minValue: UInt8 { UInt8(value: 0) }
    public static var maxValue: UInt8 { UInt8(value: 255) }
    public static var bitWidth: Int { 8 }

    public init(intLiteral value: Int) {
        self.value = lang.cast_i64_u8(value)
    }

    public func equals(other: UInt8) -> Bool {
        lang.u8_eq(self.value, other.value)
    }

    public func compare(other: UInt8) -> Ordering {
        if lang.u8_lt(self.value, other.value) { .Less }
        else if lang.u8_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](into hasher: mutating H) where H: Hasher {
        hasher.write(bytes: [self.value])
    }

    type Output = UInt8

    public func add(other: UInt8) -> UInt8 { UInt8(value: lang.u8_add(self.value, other.value)) }
    public func subtract(other: UInt8) -> UInt8 { UInt8(value: lang.u8_sub(self.value, other.value)) }
    public func multiply(other: UInt8) -> UInt8 { UInt8(value: lang.u8_mul(self.value, other.value)) }
    public func divide(other: UInt8) -> UInt8 { UInt8(value: lang.u8_div(self.value, other.value)) }
    public func mod(other: UInt8) -> UInt8 { UInt8(value: lang.u8_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt8) -> UInt8 { UInt8(value: lang.u8_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt8) -> UInt8 { UInt8(value: lang.u8_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt8) -> UInt8 { UInt8(value: lang.u8_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt8 { UInt8(value: lang.u8_not(self.value)) }
    public func shiftLeft(by count: Int) -> UInt8 { UInt8(value: lang.u8_shl(self.value, lang.cast_i64_u8(count))) }
    public func shiftRight(by count: Int) -> UInt8 { UInt8(value: lang.u8_shr(self.value, lang.cast_i64_u8(count))) }
}
