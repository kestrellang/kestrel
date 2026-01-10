// UInt64 - 64-bit unsigned integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)

public struct UInt64:
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
    private var value: lang.i64

    public static var zero: UInt64 { UInt64(intLiteral: 0) }
    public static var one: UInt64 { UInt64(intLiteral: 1) }
    public static var minValue: UInt64 { UInt64(intLiteral: 0) }
    public static var maxValue: UInt64 { UInt64(intLiteral: 18446744073709551615) }
    public static var bitWidth: Int { 64 }

    public init(intLiteral value: lang.i64) {
        self.value = value
    }

    init(raw value: lang.i64) {
        self.value = value
    }

    public func equals(other: UInt64) -> Bool {
        lang.i64_eq(self.value, other.value)
    }

    public func compare(other: UInt64) -> Ordering {
        if lang.i64_unsigned_lt(self.value, other.value) { .Less }
        else if lang.i64_unsigned_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = UInt64

    public func add(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_add(self.value, other.value)) }
    public func subtract(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_sub(self.value, other.value)) }
    public func multiply(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_mul(self.value, other.value)) }
    public func divide(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_div(self.value, other.value)) }
    public func mod(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt64 { UInt64(raw: lang.i64_not(self.value)) }
    public func shiftLeft(by count: Int) -> UInt64 { UInt64(raw: lang.i64_shl(self.value, count)) }
    public func shiftRight(by count: Int) -> UInt64 { UInt64(raw: lang.i64_unsigned_shr(self.value, count)) }
}

// UInt - platform-sized unsigned integer (alias to UInt64 on 64-bit platforms)
public type UInt = UInt64
