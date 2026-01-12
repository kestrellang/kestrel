// Int16 - 16-bit signed integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)
import std.ops.(
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral
)

public struct Int16:
    SignedInteger,
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
    Negatable,
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

    public static var zero: Int16 { Int16(intLiteral: 0) }
    public static var one: Int16 { Int16(intLiteral: 1) }
    public static var minValue: Int16 { Int16(intLiteral: -32768) }
    public static var maxValue: Int16 { Int16(intLiteral: 32767) }
    public static var bitWidth: Int { 16 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i16(value)
    }

    init(raw value: lang.i16) {
        self.value = value
    }

    public func equals(other: Int16) -> Bool {
        lang.i16_eq(self.value, other.value)
    }

    public func compare(other: Int16) -> Ordering {
        if lang.i16_signed_lt(self.value, other.value) { .Less }
        else if lang.i16_signed_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = Int16

    public func add(other: Int16) -> Int16 { Int16(raw: lang.i16_add(self.value, other.value)) }
    public func subtract(other: Int16) -> Int16 { Int16(raw: lang.i16_sub(self.value, other.value)) }
    public func multiply(other: Int16) -> Int16 { Int16(raw: lang.i16_mul(self.value, other.value)) }
    public func divide(other: Int16) -> Int16 { Int16(raw: lang.i16_signed_div(self.value, other.value)) }
    public func mod(other: Int16) -> Int16 { Int16(raw: lang.i16_signed_rem(self.value, other.value)) }
    public func negate() -> Int16 { Int16(raw: lang.i16_neg(self.value)) }
    public func abs() -> Int16 { if lang.i16_signed_lt(self.value, 0) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int16) -> Int16 { Int16(raw: lang.i16_and(self.value, other.value)) }
    public func bitwiseOr(other: Int16) -> Int16 { Int16(raw: lang.i16_or(self.value, other.value)) }
    public func bitwiseXor(other: Int16) -> Int16 { Int16(raw: lang.i16_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int16 { Int16(raw: lang.i16_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> Int16 { Int16(raw: lang.i16_shl(self.value, lang.cast_i64_i16(count))) }
    public func shiftRight(by count: lang.i64) -> Int16 { Int16(raw: lang.i16_signed_shr(self.value, lang.cast_i64_i16(count))) }
}
