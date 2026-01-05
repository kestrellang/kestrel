// Int8 - 8-bit signed integer
// Generated from templates/integer.ks.template

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

    // Numeric protocol
    public static var zero: Int8 { Int8(value: 0) }
    public static var one: Int8 { Int8(value: 1) }

    // Integer protocol
    public static var minValue: Int8 { Int8(value: -128) }
    public static var maxValue: Int8 { Int8(value: 127) }
    public static var bitWidth: Int { 8 }

    // ExpressibleByIntLiteral
    public init(intLiteral value: Int) {
        self.value = value as lang.i8
    }

    // Equatable
    public func equals(other: Int8) -> Bool {
        lang.i8_eq(self.value, other.value)
    }

    // Comparable
    public func compare(other: Int8) -> Ordering {
        if lang.i8_lt(self.value, other.value) {
            .Less
        } else if lang.i8_gt(self.value, other.value) {
            .Greater
        } else {
            .Equal
        }
    }

    // Hashable
    public func hash[H: Hasher](into hasher: ref H) {
        hasher.write(bytes: [self.value as UInt8])
    }

    // Addable
    type Output = Int8

    public func add(other: Int8) -> Int8 {
        Int8(value: lang.i8_add(self.value, other.value))
    }

    // Subtractable
    public func subtract(other: Int8) -> Int8 {
        Int8(value: lang.i8_sub(self.value, other.value))
    }

    // Multipliable
    public func multiply(other: Int8) -> Int8 {
        Int8(value: lang.i8_mul(self.value, other.value))
    }

    // Divisible
    public func divide(other: Int8) -> Int8 {
        Int8(value: lang.i8_div(self.value, other.value))
    }

    // Modulo
    public func mod(other: Int8) -> Int8 {
        Int8(value: lang.i8_rem(self.value, other.value))
    }

    // Negatable
    public func negate() -> Int8 {
        Int8(value: lang.i8_neg(self.value))
    }

    // SignedInteger
    public func abs() -> Int8 {
        if lang.i8_lt(self.value, 0) {
            Int8(value: lang.i8_neg(self.value))
        } else {
            self
        }
    }

    // BitwiseAnd
    public func bitwiseAnd(other: Int8) -> Int8 {
        Int8(value: lang.i8_and(self.value, other.value))
    }

    // BitwiseOr
    public func bitwiseOr(other: Int8) -> Int8 {
        Int8(value: lang.i8_or(self.value, other.value))
    }

    // BitwiseXor
    public func bitwiseXor(other: Int8) -> Int8 {
        Int8(value: lang.i8_xor(self.value, other.value))
    }

    // BitwiseNot
    public func bitwiseNot() -> Int8 {
        Int8(value: lang.i8_not(self.value))
    }

    // LeftShift
    public func shiftLeft(by count: Int) -> Int8 {
        Int8(value: lang.i8_shl(self.value, count as lang.i8))
    }

    // RightShift (arithmetic)
    public func shiftRight(by count: Int) -> Int8 {
        Int8(value: lang.i8_shr(self.value, count as lang.i8))
    }

    // Type conversions
    public func toInt() -> Int {
        Int64(value: self.value as lang.i64)
    }

    public func toInt16() -> Int16 {
        Int16(value: self.value as lang.i16)
    }

    public func toInt32() -> Int32 {
        Int32(value: self.value as lang.i32)
    }

    public func toInt64() -> Int64 {
        Int64(value: self.value as lang.i64)
    }

    public func toFloat32() -> Float32 {
        Float32(value: self.value as lang.f32)
    }

    public func toFloat64() -> Float64 {
        Float64(value: self.value as lang.f64)
    }
}
