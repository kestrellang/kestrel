// UInt64 - 64-bit unsigned integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable, Formattable, Hash, Hasher,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    ExpressibleByIntLiteral, Convertible
)
import std.text.(String)
import std.memory.(Slice, Pointer)
import std.num.(UInt8, Int64)

public struct UInt64:
    UnsignedInteger,
    Steppable,
    Comparable,
    Equatable,
    Matchable,
    Formattable,
    Hash,
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
    FFISafe,
        Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32]
{
    public var raw: lang.i64

    public static var zero: UInt64 { UInt64(intLiteral: 0) }
    public static var one: UInt64 { UInt64(intLiteral: 1) }
    public static var minValue: UInt64 { UInt64(intLiteral: lang.i64_neg(0)) }
    public static var maxValue: UInt64 { UInt64(intLiteral: 18446744073709551615) }
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
    public init(from other: Int64) { self.raw = other.raw }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i64(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i64(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i64(other.raw) }

    public func equals(other: UInt64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    public func matches(other: UInt64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    public func compare(other: UInt64) -> Ordering {
        if Bool(boolLiteral: lang.i64_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i64_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    public func successor() -> UInt64 { self.add(UInt64.one) }
    public func predecessor() -> UInt64 { self.subtract(UInt64.one) }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: 8)))
    }

    // Associated type bindings
    type Addable.Output = UInt64
    type Subtractable.Output = UInt64
    type Multipliable.Output = UInt64
    type Divisible.Output = UInt64
    type Modulo.Output = UInt64
    
    type BitwiseAnd.Output = UInt64
    type BitwiseOr.Output = UInt64
    type BitwiseXor.Output = UInt64
    type BitwiseNot.Output = UInt64
    type LeftShift.Output = UInt64
    type RightShift.Output = UInt64

    public func add(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_add(self.raw, other.raw)) }
    public func subtract(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_sub(self.raw, other.raw)) }
    public func multiply(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_mul(self.raw, other.raw)) }
    public func divide(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_div(self.raw, other.raw)) }
    public func modulo(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_rem(self.raw, other.raw)) }
    
    
    public func bitwiseAnd(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_and(self.raw, other.raw)) }
    public func bitwiseOr(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_or(self.raw, other.raw)) }
    public func bitwiseXor(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> UInt64 { UInt64(raw: lang.i64_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_shl(self.raw, count)) }
    public func shiftRight(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_unsigned_shr(self.raw, count)) }

    // Formattable
    public func format() -> String {
        if self == UInt64.zero {
            return "0"
        }

        var result = String();
        var n = self;

        let ten: UInt64 = 10;
        while n != UInt64.zero {
            let digit: UInt64 = n % ten;
            let charCode: Int64 = Int64(from: digit) + 48;
            result.appendByte(UInt8(from: charCode));
            n = n / ten
        }

        // Reverse the string
        var reversed = String();
        var i = result.byteCount() - 1;
        while i >= 0 {
            reversed.appendByte(result.byteAtUnchecked(i));
            i = i - 1
        }
        reversed
    }}

// UInt - platform-sized unsigned integer (alias to UInt64 on 64-bit platforms)
public type UInt = UInt64
