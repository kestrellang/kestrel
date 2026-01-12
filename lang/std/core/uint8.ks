// UInt8 - 8-bit unsigned integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)
import std.ops.(
    Addable, Subtractable, Multipliable, Divisible, Modulo,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral
)

public struct UInt8:
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
    private var value: lang.i8

    public static var zero: UInt8 { UInt8(intLiteral: 0) }
    public static var one: UInt8 { UInt8(intLiteral: 1) }
    public static var minValue: UInt8 { UInt8(intLiteral: 0) }
    public static var maxValue: UInt8 { UInt8(intLiteral: 255) }
    public static var bitWidth: Int { 8 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i8(value)
    }

    init(raw value: lang.i8) {
        self.value = value
    }

    public func equals(other: UInt8) -> Bool {
        lang.i8_eq(self.value, other.value)
    }

    public func compare(other: UInt8) -> Ordering {
        if lang.i8_unsigned_lt(self.value, other.value) { .Less }
        else if lang.i8_unsigned_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: [self.value])
    }

    type Output = UInt8

    public func add(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_add(self.value, other.value)) }
    public func subtract(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_sub(self.value, other.value)) }
    public func multiply(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_mul(self.value, other.value)) }
    public func divide(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_unsigned_div(self.value, other.value)) }
    public func mod(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_unsigned_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt8 { UInt8(raw: lang.i8_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> UInt8 { UInt8(raw: lang.i8_shl(self.value, lang.cast_i64_i8(count))) }
    public func shiftRight(by count: lang.i64) -> UInt8 { UInt8(raw: lang.i8_unsigned_shr(self.value, lang.cast_i64_i8(count))) }
}
