// Int16 - 16-bit signed integer
// Generated from templates/integer.ks.template

import std.ffi.(FFISafe)

public struct Int16:
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
    private var value: lang.i16

    public static var zero: Int16 { Int16(value: 0) }
    public static var one: Int16 { Int16(value: 1) }
    public static var minValue: Int16 { Int16(value: -32768) }
    public static var maxValue: Int16 { Int16(value: 32767) }
    public static var bitWidth: Int { 16 }

    public init(intLiteral value: Int) {
        self.value = lang.cast_i64_i16(value)
    }

    public func equals(other: Int16) -> Bool {
        lang.i16_eq(self.value, other.value)
    }

    public func compare(other: Int16) -> Ordering {
        if lang.i16_lt(self.value, other.value) { .Less }
        else if lang.i16_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    public func hash[H](into hasher: mutating H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    type Output = Int16

    public func add(other: Int16) -> Int16 { Int16(value: lang.i16_add(self.value, other.value)) }
    public func subtract(other: Int16) -> Int16 { Int16(value: lang.i16_sub(self.value, other.value)) }
    public func multiply(other: Int16) -> Int16 { Int16(value: lang.i16_mul(self.value, other.value)) }
    public func divide(other: Int16) -> Int16 { Int16(value: lang.i16_div(self.value, other.value)) }
    public func mod(other: Int16) -> Int16 { Int16(value: lang.i16_rem(self.value, other.value)) }
    public func negate() -> Int16 { Int16(value: lang.i16_neg(self.value)) }
    public func abs() -> Int16 { if lang.i16_lt(self.value, 0) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int16) -> Int16 { Int16(value: lang.i16_and(self.value, other.value)) }
    public func bitwiseOr(other: Int16) -> Int16 { Int16(value: lang.i16_or(self.value, other.value)) }
    public func bitwiseXor(other: Int16) -> Int16 { Int16(value: lang.i16_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int16 { Int16(value: lang.i16_not(self.value)) }
    public func shiftLeft(by count: Int) -> Int16 { Int16(value: lang.i16_shl(self.value, lang.cast_i64_i16(count))) }
    public func shiftRight(by count: Int) -> Int16 { Int16(value: lang.i16_shr(self.value, lang.cast_i64_i16(count))) }

    // Type conversions
    public func toInt() -> Int {
        Int64(value: lang.cast_i16_i64(self.value))
    }

    public func toInt8() -> Int8 {
        Int8(value: lang.cast_i16_i8(self.value))
    }

    public func toInt32() -> Int32 {
        Int32(value: lang.cast_i16_i32(self.value))
    }

    public func toInt64() -> Int64 {
        Int64(value: lang.cast_i16_i64(self.value))
    }

    public func toFloat32() -> Float32 {
        Float32(value: lang.cast_i16_f32(self.value))
    }

    public func toFloat64() -> Float64 {
        Float64(value: lang.cast_i16_f64(self.value))
    }
}
