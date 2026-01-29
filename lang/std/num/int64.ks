// Int64 - 64-bit signed integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable, Formattable, Hash, Hasher,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    AddAssign, SubtractAssign, MultiplyAssign, DivideAssign, ModuloAssign,
    BitwiseAndAssign, BitwiseOrAssign, BitwiseXorAssign, LeftShiftAssign, RightShiftAssign,
    ExpressibleByIntLiteral, Convertible
)
import std.text.(String)
import std.memory.(Slice, Pointer)
import std.num.(UInt8, Int64)

public struct Int64:
    SignedInteger,
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
    Negatable,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseNot,
    LeftShift,
    RightShift,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
    LeftShiftAssign[lang.i64],
    RightShiftAssign[lang.i64],
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

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    public static var zero: Int64 { Int64(intLiteral: 0) }
    public static var one: Int64 { Int64(intLiteral: 1) }
    public static var minValue: Int64 { Int64(intLiteral: lang.i64_neg(9223372036854775808)) }
    public static var maxValue: Int64 { Int64(intLiteral: 9223372036854775807) }
    public static var bitWidth: Int64 { Int64(intLiteral: 64) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

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

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: Int64 { get {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { Int64(intLiteral: lang.i64_neg(1)) }
        else if Bool(boolLiteral: lang.i64_eq(self.raw, 0)) { Int64.zero }
        else { Int64.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i64_signed_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0))
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i64_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 1)) { false }
        else { Bool(boolLiteral: lang.i64_eq(lang.i64_and(self.raw, lang.i64_sub(self.raw, 1)), 0)) }
    }}

    // TODO: requires lang.i64_popcount intrinsic
    public var countOnes: Int64 { get {
        // Stub implementation - counts bits manually
        var count: Int64 = 0;
        var n = self.raw;
        var i: Int64 = 0;
        while i < 64 {
            if not Bool(boolLiteral: lang.i64_eq(lang.i64_and(n, 1), 0)) {
                count = count + 1
            };
            n = lang.i64_signed_shr(n, 1);
            i = i + 1
        };
        count
    }}

    public var countZeros: Int64 { get {
        Int64(intLiteral: 64) - self.countOnes
    }}

    // TODO: requires lang.i64_clz intrinsic
    public var leadingZeros: Int64 { get {
        if self == Int64.zero {
            return Int64(intLiteral: 64)
        };
        var count: Int64 = 0;
        var n = self.raw;
        var i: Int64 = 64 - 1;
        while i >= 0 {
            let bit = lang.i64_and(lang.i64_signed_shr(n, i.raw), 1);
            if not Bool(boolLiteral: lang.i64_eq(bit, 0)) {
                return count
            };
            count = count + 1;
            i = i - 1
        };
        count
    }}

    // TODO: requires lang.i64_ctz intrinsic
    public var trailingZeros: Int64 { get {
        if self == Int64.zero {
            return Int64(intLiteral: 64)
        };
        var count: Int64 = 0;
        var n = self.raw;
        while Bool(boolLiteral: lang.i64_eq(lang.i64_and(n, 1), 0)) {
            count = count + 1;
            n = lang.i64_signed_shr(n, 1)
        };
        count
    }}

    // TODO: requires lang.i64_bswap intrinsic
    public var byteSwapped: Int64 { get {
        // Swap bytes
        let b0 = lang.i64_and(self.raw, 255);
        let b1 = lang.i64_and(lang.i64_signed_shr(self.raw, 8), 255);
        let b2 = lang.i64_and(lang.i64_signed_shr(self.raw, 16), 255);
        let b3 = lang.i64_and(lang.i64_signed_shr(self.raw, 24), 255);
        let b4 = lang.i64_and(lang.i64_signed_shr(self.raw, 32), 255);
        let b5 = lang.i64_and(lang.i64_signed_shr(self.raw, 40), 255);
        let b6 = lang.i64_and(lang.i64_signed_shr(self.raw, 48), 255);
        let b7 = lang.i64_and(lang.i64_signed_shr(self.raw, 56), 255);
        Int64(raw: lang.i64_or(lang.i64_or(lang.i64_or(lang.i64_or(lang.i64_or(lang.i64_or(lang.i64_or(
            lang.i64_shl(b0, 56),
            lang.i64_shl(b1, 48)),
            lang.i64_shl(b2, 40)),
            lang.i64_shl(b3, 32)),
            lang.i64_shl(b4, 24)),
            lang.i64_shl(b5, 16)),
            lang.i64_shl(b6, 8)),
            b7))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

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

    // ========================================================================
    // STEPPING
    // ========================================================================

    public func successor() -> Int64 { self.add(Int64.one) }
    public func predecessor() -> Int64 { self.subtract(Int64.one) }

    // ========================================================================
    // HASHING
    // ========================================================================

    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[Int64]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

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

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    public func add(other: Int64) -> Int64 { Int64(raw: lang.i64_add(self.raw, other.raw)) }
    public func subtract(other: Int64) -> Int64 { Int64(raw: lang.i64_sub(self.raw, other.raw)) }
    public func multiply(other: Int64) -> Int64 { Int64(raw: lang.i64_mul(self.raw, other.raw)) }
    public func divide(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_div(self.raw, other.raw)) }
    public func modulo(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_rem(self.raw, other.raw)) }
    public func negate() -> Int64 { Int64(raw: lang.i64_neg(self.raw)) }
    public func abs() -> Int64 { if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: Int64) -> Int64? {
        // Simplified check - detect if signs are same and result sign differs
        let result = self.add(other);
        if self.isPositive and other.isPositive and result.isNegative {
            return .None
        };
        if self.isNegative and other.isNegative and result.isPositive {
            return .None
        };
        .Some(result)
    }

    public func subtractChecked(other: Int64) -> Int64? {
        // Simplified check
        let result = self.subtract(other);
        if self.isPositive and other.isNegative and result.isNegative {
            return .None
        };
        if self.isNegative and other.isPositive and result.isPositive {
            return .None
        };
        .Some(result)
    }

    public func multiplyChecked(other: Int64) -> Int64? {
        if other == Int64.zero {
            return .Some(Int64.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: Int64) -> Int64? {
        if other == Int64.zero {
            return .None
        };
        // Check for minValue / -1 overflow
        if self == Int64.minValue and other == Int64(intLiteral: lang.i64_neg(1)) {
            return .None
        };
        .Some(self.divide(other))
    }

    public func negateChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.negate())
    }

    public func absChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.abs())
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: Int64) -> Int64 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isPositive { Int64.maxValue } else { Int64.minValue }
        }
    }

    public func subtractSaturating(other: Int64) -> Int64 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isNegative { Int64.maxValue } else { Int64.minValue }
        }
    }

    public func multiplySaturating(other: Int64) -> Int64 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => {
                // Determine sign of result
                let sameSign = (self.isNegative == other.isNegative);
                if sameSign { Int64.maxValue } else { Int64.minValue }
            }
        }
    }

    public func negateSaturating() -> Int64 {
        if self == Int64.minValue {
            Int64.maxValue
        } else {
            self.negate()
        }
    }

    public func absSaturating() -> Int64 {
        if self == Int64.minValue {
            Int64.maxValue
        } else {
            self.abs()
        }
    }


    // ========================================================================
    // ARITHMETIC (Extended)
    // ========================================================================

    public func pow(exponent: Int64) -> Int64 {
        if exponent < 0 {
            return Int64.zero
        };
        if exponent == 0 {
            return Int64.one
        };
        var result = Int64.one;
        var base = self;
        var exp = exponent;
        while exp > 0 {
            if exp % 2 == 1 {
                result = result.multiply(base)
            };
            base = base.multiply(base);
            exp = exp / 2
        };
        result
    }

    public func gcd(other: Int64) -> Int64 {
        var a = self.abs();
        var b = other.abs();
        while b != Int64.zero {
            let t = b;
            b = a.modulo(b);
            a = t
        };
        a
    }

    public func lcm(other: Int64) -> Int64 {
        if self == Int64.zero or other == Int64.zero {
            return Int64.zero
        };
        let g = self.gcd(other);
        self.abs().divide(g).multiply(other.abs())
    }

    // ========================================================================
    // CLAMPING
    // ========================================================================

    public func clamp(min: Int64, max: Int64) -> Int64 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    public func bitwiseAnd(other: Int64) -> Int64 { Int64(raw: lang.i64_and(self.raw, other.raw)) }
    public func bitwiseOr(other: Int64) -> Int64 { Int64(raw: lang.i64_or(self.raw, other.raw)) }
    public func bitwiseXor(other: Int64) -> Int64 { Int64(raw: lang.i64_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> Int64 { Int64(raw: lang.i64_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_shl(self.raw, count)) }
    public func shiftRight(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_signed_shr(self.raw, count)) }

    public func rotateLeft(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    public func rotateRight(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    public mutating func addAssign(other: Int64) { self = self.add(other) }
    public mutating func subtractAssign(other: Int64) { self = self.subtract(other) }
    public mutating func multiplyAssign(other: Int64) { self = self.multiply(other) }
    public mutating func divideAssign(other: Int64) { self = self.divide(other) }
    public mutating func modAssign(other: Int64) { self = self.modulo(other) }
    public mutating func bitwiseAndAssign(other: Int64) { self = self.bitwiseAnd(other) }
    public mutating func bitwiseOrAssign(other: Int64) { self = self.bitwiseOr(other) }
    public mutating func bitwiseXorAssign(other: Int64) { self = self.bitwiseXor(other) }
    public mutating func shiftLeftAssign(by count: lang.i64) { self = self.shiftLeft(by: count) }
    public mutating func shiftRightAssign(by count: lang.i64) { self = self.shiftRight(by: count) }

    // ========================================================================
    // BYTE CONVERSION
    // ========================================================================

    // TODO: implement byte conversion methods
    // These require Array from std.collections which creates circular import issues
    // public func toBytes() -> Array[UInt8]
    // public func toBytesBigEndian() -> Array[UInt8]
    // public func toBytesLittleEndian() -> Array[UInt8]
    // public static func fromBytes(bytes: Array[UInt8]) -> Int64?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> Int64?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> Int64?

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> Int64?
    // public static func parse(string: String, radix: Int64) -> Int64?

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format() -> String {
        if self == Int64.zero {
            return "0"
        }

        var result = String();
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        let ten: Int64 = 10;
        while n != Int64.zero {
            let digit: Int64 = n % ten;
            let charCode: Int64 = digit + 48;
            result.appendByte(UInt8(from: charCode));
            n = n / ten
        }

        if isNegative {
            result.appendByte(45)  // '-'
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

// Int - platform-sized signed integer (alias to Int64 on 64-bit platforms)
public type Int = Int64
