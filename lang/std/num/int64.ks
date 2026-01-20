// Int64 - 64-bit signed integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)

public struct Int64:
    SignedInteger,
    Steppable,
    Comparable,
    Equatable,
    Matchable,
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
        Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i64

    public static var zero: Int64 { Int64(intLiteral: 0) }
    public static var one: Int64 { Int64(intLiteral: 1) }
    public static var minValue: Int64 { Int64(intLiteral: lang.i64_neg(9223372036854775808)) }
    public static var maxValue: Int64 { Int64(intLiteral: 9223372036854775807) }
    // public static var bitWidth: Int { 64 }

    public init(intLiteral value: lang.i64) {
        self.raw = value
    }

    init(raw value: lang.i64) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i64(other.raw) }
    public init(from other: Int16) { self.raw = lang.cast_i16_i64(other.raw) }
    public init(from other: Int32) { self.raw = lang.cast_i32_i64(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i64(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i64(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i64(other.raw) }
    public init(from other: UInt64) { self.raw = other.raw }

    public func equals(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    public func matches(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    public func compare(other: Int64) -> Ordering {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i64_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    public func successor() -> Int64 { self.add(Int64.one) }
    public func predecessor() -> Int64 { self.subtract(Int64.one) }

    // Associated type bindings
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

    public func add(other: Int64) -> Int64 { Int64(raw: lang.i64_add(self.raw, other.raw)) }
    public func subtract(other: Int64) -> Int64 { Int64(raw: lang.i64_sub(self.raw, other.raw)) }
    public func multiply(other: Int64) -> Int64 { Int64(raw: lang.i64_mul(self.raw, other.raw)) }
    public func divide(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_div(self.raw, other.raw)) }
    public func modulo(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_rem(self.raw, other.raw)) }
    public func negate() -> Int64 { Int64(raw: lang.i64_neg(self.raw)) }
    public func abs() -> Int64 { if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { self.negate() } else { self } }
    public func bitwiseAnd(other: Int64) -> Int64 { Int64(raw: lang.i64_and(self.raw, other.raw)) }
    public func bitwiseOr(other: Int64) -> Int64 { Int64(raw: lang.i64_or(self.raw, other.raw)) }
    public func bitwiseXor(other: Int64) -> Int64 { Int64(raw: lang.i64_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> Int64 { Int64(raw: lang.i64_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_shl(self.raw, count)) }
    public func shiftRight(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_signed_shr(self.raw, count)) }
}

// Int - platform-sized signed integer (alias to Int64 on 64-bit platforms)
public type Int = Int64
