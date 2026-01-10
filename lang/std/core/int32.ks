// Int32 - 32-bit signed integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)
import std.ops.(
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral
)

public struct Int32:
    SignedInteger,
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
    private var value: lang.i32

    public static var zero: Int32 { Int32(intLiteral: 0) }
    public static var one: Int32 { Int32(intLiteral: 1) }
    public static var minValue: Int32 { Int32(intLiteral: -2147483648) }
    public static var maxValue: Int32 { Int32(intLiteral: 2147483647) }
    public static var bitWidth: Int { 32 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i32(value)
    }

    // Internal init for raw lang type values
    init(raw value: lang.i32) {
        self.value = value
    }

    public func equals(other: Int32) -> Bool {
        lang.i32_eq(self.value, other.value)
    }

    public func compare(other: Int32) -> Ordering {
        if lang.i32_signed_lt(self.value, other.value) { .Less }
        else if lang.i32_signed_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = Int32

    public func add(other: Int32) -> Int32 { Int32(raw: lang.i32_add(self.value, other.value)) }
    public func subtract(other: Int32) -> Int32 { Int32(raw: lang.i32_sub(self.value, other.value)) }
    public func multiply(other: Int32) -> Int32 { Int32(raw: lang.i32_mul(self.value, other.value)) }
    public func divide(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_div(self.value, other.value)) }
    public func mod(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_rem(self.value, other.value)) }
    public func negate() -> Int32 { Int32(raw: lang.i32_neg(self.value)) }
    public func abs() -> Int32 { if lang.i32_signed_lt(self.value, 0) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int32) -> Int32 { Int32(raw: lang.i32_and(self.value, other.value)) }
    public func bitwiseOr(other: Int32) -> Int32 { Int32(raw: lang.i32_or(self.value, other.value)) }
    public func bitwiseXor(other: Int32) -> Int32 { Int32(raw: lang.i32_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int32 { Int32(raw: lang.i32_not(self.value)) }
    public func shiftLeft(by count: Int) -> Int32 { Int32(raw: lang.i32_shl(self.value, lang.cast_i64_i32(count))) }
    public func shiftRight(by count: Int) -> Int32 { Int32(raw: lang.i32_signed_shr(self.value, lang.cast_i64_i32(count))) }
}
