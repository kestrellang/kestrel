// Int64 - 64-bit signed integer
// Generated from templates/integer.ks.template

import std.ffi.(FFISafe)

public struct Int64:
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
    private var value: lang.i64

    public static var zero: Int64 { Int64(value: 0) }
    public static var one: Int64 { Int64(value: 1) }
    public static var minValue: Int64 { Int64(value: -9223372036854775808) }
    public static var maxValue: Int64 { Int64(value: 9223372036854775807) }
    public static var bitWidth: Int { 64 }

    public init(intLiteral value: Int) {
        self.value = value as lang.i64
    }

    public func equals(other: Int64) -> Bool {
        lang.i64_eq(self.value, other.value)
    }

    public func compare(other: Int64) -> Ordering {
        if lang.i64_lt(self.value, other.value) { .Less }
        else if lang.i64_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](into hasher: mutating H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = Int64

    public func add(other: Int64) -> Int64 { Int64(value: lang.i64_add(self.value, other.value)) }
    public func subtract(other: Int64) -> Int64 { Int64(value: lang.i64_sub(self.value, other.value)) }
    public func multiply(other: Int64) -> Int64 { Int64(value: lang.i64_mul(self.value, other.value)) }
    public func divide(other: Int64) -> Int64 { Int64(value: lang.i64_div(self.value, other.value)) }
    public func mod(other: Int64) -> Int64 { Int64(value: lang.i64_rem(self.value, other.value)) }
    public func negate() -> Int64 { Int64(value: lang.i64_neg(self.value)) }
    public func abs() -> Int64 { if lang.i64_lt(self.value, 0) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int64) -> Int64 { Int64(value: lang.i64_and(self.value, other.value)) }
    public func bitwiseOr(other: Int64) -> Int64 { Int64(value: lang.i64_or(self.value, other.value)) }
    public func bitwiseXor(other: Int64) -> Int64 { Int64(value: lang.i64_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int64 { Int64(value: lang.i64_not(self.value)) }
    public func shiftLeft(by count: Int) -> Int64 { Int64(value: lang.i64_shl(self.value, count as lang.i64)) }
    public func shiftRight(by count: Int) -> Int64 { Int64(value: lang.i64_shr(self.value, count as lang.i64)) }

    // Type conversions
    public func toInt8() -> Int8 {
        Int8(value: self.value as lang.i8)
    }

    public func toInt16() -> Int16 {
        Int16(value: self.value as lang.i16)
    }

    public func toInt32() -> Int32 {
        Int32(value: self.value as lang.i32)
    }

    public func toFloat32() -> Float32 {
        Float32(value: self.value as lang.f32)
    }

    public func toFloat64() -> Float64 {
        Float64(value: self.value as lang.f64)
    }
}

// Int - platform-sized signed integer (alias to Int64 on 64-bit platforms)
public type Int = Int64
