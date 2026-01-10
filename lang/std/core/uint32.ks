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
    private var value: lang.i32

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

    public func equals(other: UInt32) -> Bool {
        lang.i32_eq(self.value, other.value)
    }

    public func compare(other: UInt32) -> Ordering {
        if lang.i32_unsigned_lt(self.value, other.value) { .Less }
        else if lang.i32_unsigned_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = UInt32

    public func add(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_add(self.value, other.value)) }
    public func subtract(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_sub(self.value, other.value)) }
    public func multiply(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_mul(self.value, other.value)) }
    public func divide(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_div(self.value, other.value)) }
    public func mod(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt32 { UInt32(raw: lang.i32_not(self.value)) }
    public func shiftLeft(by count: Int) -> UInt32 { UInt32(raw: lang.i32_shl(self.value, lang.cast_i64_i32(count))) }
    public func shiftRight(by count: Int) -> UInt32 { UInt32(raw: lang.i32_unsigned_shr(self.value, lang.cast_i64_i32(count))) }
}
