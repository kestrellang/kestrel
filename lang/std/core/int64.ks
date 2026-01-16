// Int64 - 64-bit signed integer
// Generated from templates/integer.ks.template

module std.core

import std.ffi.(FFISafe)
import std.ops.(
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral
)

public struct Int64:
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
    FFISafe,
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64],
    Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32]
{
    private var value: lang.i64

    public var raw: lang.i64 { self.value }

    public static var zero: Int64 { Int64(intLiteral: 0) }
    public static var one: Int64 { Int64(intLiteral: 1) }
    public static var minValue: Int64 { Int64(intLiteral: -9223372036854775808) }
    public static var maxValue: Int64 { Int64(intLiteral: 9223372036854775807) }
    public static var bitWidth: Int { 64 }

    public init(intLiteral value: lang.i64) {
        self.value = value
    }

    // Internal init for raw lang type values
    init(raw value: lang.i64) {
        self.value = value
    }

    // Conversions from other integer types
    public init(from other: UInt8) { self.value = lang.cast_i8_i64(other.raw) }
    public init(from other: UInt16) { self.value = lang.cast_i16_i64(other.raw) }
    public init(from other: UInt32) { self.value = lang.cast_i32_i64(other.raw) }
    public init(from other: UInt64) { self.value = other.raw }
    public init(from other: Int8) { self.value = lang.cast_i8_i64(other.raw) }
    public init(from other: Int16) { self.value = lang.cast_i16_i64(other.raw) }
    public init(from other: Int32) { self.value = lang.cast_i32_i64(other.raw) }

    public func equals(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.value, other.value))
    }

    public func compare(other: Int64) -> Ordering {
        if Bool(boolLiteral: lang.i64_signed_lt(self.value, other.value)) { .Less }
        else if Bool(boolLiteral: lang.i64_signed_gt(self.value, other.value)) { .Greater }
        else { .Equal }
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(bytes: self.value.toBytes())
    }

    // Associated type bindings (qualified to avoid ambiguity across protocols)
    type Addable.Output = Int64
    type Subtractable.Output = Int64
    type Multipliable.Output = Int64
    type Divisible.Output = Int64
    type Modulo.Output = Int64
    type Negatable.Output = Int64
    type BitwiseAnd.Output = Int64
    type BitwiseOr.Output = Int64
    type BitwiseXor.Output = Int64
    type BitwiseNot.Output = Int64
    type LeftShift.Output = Int64
    type RightShift.Output = Int64

    public func add(other: Int64) -> Int64 { Int64(raw: lang.i64_add(self.value, other.value)) }
    public func subtract(other: Int64) -> Int64 { Int64(raw: lang.i64_sub(self.value, other.value)) }
    public func multiply(other: Int64) -> Int64 { Int64(raw: lang.i64_mul(self.value, other.value)) }
    public func divide(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_div(self.value, other.value)) }
    public func modulo(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_rem(self.value, other.value)) }
    public func negate() -> Int64 { Int64(raw: lang.i64_neg(self.value)) }
    public func abs() -> Int64 { if Bool(boolLiteral: lang.i64_signed_lt(self.value, 0)) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int64) -> Int64 { Int64(raw: lang.i64_and(self.value, other.value)) }
    public func bitwiseOr(other: Int64) -> Int64 { Int64(raw: lang.i64_or(self.value, other.value)) }
    public func bitwiseXor(other: Int64) -> Int64 { Int64(raw: lang.i64_xor(self.value, other.value)) }
    public func bitwiseNot() -> Int64 { Int64(raw: lang.i64_not(self.value)) }
    public func shiftLeft(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_shl(self.value, count)) }
    public func shiftRight(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_signed_shr(self.value, count)) }
}

// Int - platform-sized signed integer (alias to Int64 on 64-bit platforms)
public type Int = Int64
