// Int8 - 8-bit signed integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)

public struct Int8:
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
    private var value: lang.i8

    public static var zero: Int8 { Int8(intLiteral: 0) }
    public static var one: Int8 { Int8(intLiteral: 1) }
    public static var minValue: Int8 { Int8(intLiteral: -128) }
    public static var maxValue: Int8 { Int8(intLiteral: 127) }
    public static var bitWidth: Int { 8 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_i8(value)
    }

    init(raw value: lang.i8) {
        self.value = value
    }

    public func equals(other: Int8) -> Bool {
        lang.i8_eq(self.value, other.value)
    }

    public func compare(other: Int8) -> Ordering {
        if lang.i8_signed_lt(self.value, other.value) { .Less }
        else if lang.i8_signed_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: [self.value])
    }

    type Output = Int8

    public func add(other: Int8) -> Int8 { Int8(raw: lang.i8_add(self.value, other.value)) }
    public func subtract(other: Int8) -> Int8 { Int8(raw: lang.i8_sub(self.value, other.value)) }
    public func multiply(other: Int8) -> Int8 { Int8(raw: lang.i8_mul(self.value, other.value)) }
    public func divide(other: Int8) -> Int8 { Int8(raw: lang.i8_signed_div(self.value, other.value)) }
    public func mod(other: Int8) -> Int8 { Int8(raw: lang.i8_signed_rem(self.value, other.value)) }
    public func negate() -> Int8 { Int8(raw: lang.i8_neg(self.value)) }
    public func abs() -> Int8 { if lang.i8_signed_lt(self.value, 0) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int8) -> Int8 { Int8(raw: lang.i8_and(self.value, other.value)) }
    public func bitwiseOr(other: Int8) -> Int8 { Int8(raw: lang.i8_or(self.value, other.value)) }
    public func bitwiseXor(other: Int8) -> Int8 { Int8(raw: lang.i8_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int8 { Int8(raw: lang.i8_not(self.value)) }
    public func shiftLeft(by count: Int) -> Int8 { Int8(raw: lang.i8_shl(self.value, lang.cast_i64_i8(count))) }
    public func shiftRight(by count: Int) -> Int8 { Int8(raw: lang.i8_signed_shr(self.value, lang.cast_i64_i8(count))) }
}
