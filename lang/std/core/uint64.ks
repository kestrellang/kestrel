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
    private var value: lang.u64

    public static var zero: UInt64 { UInt64(value: 0) }
    public static var one: UInt64 { UInt64(value: 1) }
    public static var minValue: UInt64 { UInt64(value: 0) }
    public static var maxValue: UInt64 { UInt64(value: 18446744073709551615) }
    public static var bitWidth: Int { 64 }

    public init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_u64(value)
    }

    public func equals(other: UInt64) -> Bool {
        lang.u64_eq(self.value, other.value)
    }

    public func compare(other: UInt64) -> Ordering {
        if lang.u64_lt(self.value, other.value) { .Less }
        else if lang.u64_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](into hasher: mutating H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = UInt64

    public func add(other: UInt64) -> UInt64 { UInt64(value: lang.u64_add(self.value, other.value)) }
    public func subtract(other: UInt64) -> UInt64 { UInt64(value: lang.u64_sub(self.value, other.value)) }
    public func multiply(other: UInt64) -> UInt64 { UInt64(value: lang.u64_mul(self.value, other.value)) }
    public func divide(other: UInt64) -> UInt64 { UInt64(value: lang.u64_div(self.value, other.value)) }
    public func mod(other: UInt64) -> UInt64 { UInt64(value: lang.u64_rem(self.value, other.value)) }
    public func bitwiseAnd(other: UInt64) -> UInt64 { UInt64(value: lang.u64_and(self.value, other.value)) }
    public func bitwiseOr(other: UInt64) -> UInt64 { UInt64(value: lang.u64_or(self.value, other.value)) }
    public func bitwiseXor(other: UInt64) -> UInt64 { UInt64(value: lang.u64_xor(self.value, other.value)) }
    public func bitwiseNot() -> UInt64 { UInt64(value: lang.u64_not(self.value)) }
    public func shiftLeft(by count: Int) -> UInt64 { UInt64(value: lang.u64_shl(self.value, lang.cast_i64_u64(count))) }
    public func shiftRight(by count: Int) -> UInt64 { UInt64(value: lang.u64_shr(self.value, lang.cast_i64_u64(count))) }

    // Helper to convert to bytes for hashing
    public func toBytes() -> [UInt8] {
        [
            UInt8(self.value & 0xFF),
            UInt8((self.value >> 8) & 0xFF),
            UInt8((self.value >> 16) & 0xFF),
            UInt8((self.value >> 24) & 0xFF),
            UInt8((self.value >> 32) & 0xFF),
            UInt8((self.value >> 40) & 0xFF),
            UInt8((self.value >> 48) & 0xFF),
            UInt8((self.value >> 56) & 0xFF)
        ]
    }
}

// UInt - platform-sized unsigned integer (alias to UInt64 on 64-bit platforms)
public type UInt = UInt64
