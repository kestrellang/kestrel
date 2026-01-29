// Int16 - 16-bit signed integer
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

public struct Int16:
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
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i16

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    public static var zero: Int16 { Int16(intLiteral: 0) }
    public static var one: Int16 { Int16(intLiteral: 1) }
    public static var minValue: Int16 { Int16(intLiteral: lang.i64_neg(32768)) }
    public static var maxValue: Int16 { Int16(intLiteral: 32767) }
    public static var bitWidth: Int64 { Int64(intLiteral: 16) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i16(value)
    }

    init(raw value: lang.i16) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i16(other.raw) }
    public init(from other: Int32) { self.raw = lang.cast_i32_i16(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i16(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i16(other.raw) }
    public init(from other: UInt16) { self.raw = other.raw }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i16(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i16(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: Int16 { get {
        if Bool(boolLiteral: lang.i16_signed_lt(self.raw, 0)) { Int16(intLiteral: lang.i64_neg(1)) }
        else if Bool(boolLiteral: lang.i16_eq(self.raw, 0)) { Int16.zero }
        else { Int16.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i16_signed_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        Bool(boolLiteral: lang.i16_signed_lt(self.raw, 0))
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i16_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i16_signed_lt(self.raw, 1)) { false }
        else { Bool(boolLiteral: lang.i16_eq(lang.i16_and(self.raw, lang.i16_sub(self.raw, 1)), 0)) }
    }}

    // TODO: requires lang.i16_popcount intrinsic
    public var countOnes: Int64 { get {
        // Stub implementation - counts bits manually
        var count: Int64 = 0;
        var n = self.raw;
        var i: Int64 = 0;
        while i < 16 {
            if not Bool(boolLiteral: lang.i16_eq(lang.i16_and(n, 1), 0)) {
                count = count + 1
            };
            n = lang.i16_signed_shr(n, 1);
            i = i + 1
        };
        count
    }}

    public var countZeros: Int64 { get {
        Int64(intLiteral: 16) - self.countOnes
    }}

    // TODO: requires lang.i16_clz intrinsic
    public var leadingZeros: Int64 { get {
        if self == Int16.zero {
            return Int64(intLiteral: 16)
        };
        var count: Int64 = 0;
        var n = self.raw;
        var i: Int64 = 16 - 1;
        while i >= 0 {
            let bit = lang.i16_and(lang.i16_signed_shr(n, lang.cast_i64_i16(i.raw)), 1);
            if not Bool(boolLiteral: lang.i16_eq(bit, 0)) {
                return count
            };
            count = count + 1;
            i = i - 1
        };
        count
    }}

    // TODO: requires lang.i16_ctz intrinsic
    public var trailingZeros: Int64 { get {
        if self == Int16.zero {
            return Int64(intLiteral: 16)
        };
        var count: Int64 = 0;
        var n = self.raw;
        while Bool(boolLiteral: lang.i16_eq(lang.i16_and(n, 1), 0)) {
            count = count + 1;
            n = lang.i16_signed_shr(n, 1)
        };
        count
    }}

    // TODO: requires lang.i16_bswap intrinsic
    public var byteSwapped: Int16 { get {
        Int16(raw: lang.i16_or(
            lang.i16_shl(lang.i16_and(self.raw, 255), 8),
            lang.i16_and(lang.i16_signed_shr(self.raw, 8), 255)
        ))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    public func equals(other: Int16) -> Bool {
        Bool(boolLiteral: lang.i16_eq(self.raw, other.raw))
    }

    public func matches(other: Int16) -> Bool {
        Bool(boolLiteral: lang.i16_eq(self.raw, other.raw))
    }

    public func compare(other: Int16) -> Ordering {
        if Bool(boolLiteral: lang.i16_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i16_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    public func successor() -> Int16 { self.add(Int16.one) }
    public func predecessor() -> Int16 { self.subtract(Int16.one) }

    // ========================================================================
    // HASHING
    // ========================================================================

    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[Int16]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = Int16
    type Subtractable.Output = Int16
    type Multipliable.Output = Int16
    type Divisible.Output = Int16
    type Modulo.Output = Int16
    type Negatable.Output = Int16
    type BitwiseAnd.Output = Int16
    type BitwiseOr.Output = Int16
    type BitwiseXor.Output = Int16
    type BitwiseNot.Output = Int16
    type LeftShift.Output = Int16
    type RightShift.Output = Int16

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    public func add(other: Int16) -> Int16 { Int16(raw: lang.i16_add(self.raw, other.raw)) }
    public func subtract(other: Int16) -> Int16 { Int16(raw: lang.i16_sub(self.raw, other.raw)) }
    public func multiply(other: Int16) -> Int16 { Int16(raw: lang.i16_mul(self.raw, other.raw)) }
    public func divide(other: Int16) -> Int16 { Int16(raw: lang.i16_signed_div(self.raw, other.raw)) }
    public func modulo(other: Int16) -> Int16 { Int16(raw: lang.i16_signed_rem(self.raw, other.raw)) }
    public func negate() -> Int16 { Int16(raw: lang.i16_neg(self.raw)) }
    public func abs() -> Int16 { if Bool(boolLiteral: lang.i16_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: Int16) -> Int16? {
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

    public func subtractChecked(other: Int16) -> Int16? {
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

    public func multiplyChecked(other: Int16) -> Int16? {
        if other == Int16.zero {
            return .Some(Int16.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: Int16) -> Int16? {
        if other == Int16.zero {
            return .None
        };
        // Check for minValue / -1 overflow
        if self == Int16.minValue and other == Int16(intLiteral: lang.i64_neg(1)) {
            return .None
        };
        .Some(self.divide(other))
    }

    public func negateChecked() -> Int16? {
        if self == Int16.minValue {
            return .None
        };
        .Some(self.negate())
    }

    public func absChecked() -> Int16? {
        if self == Int16.minValue {
            return .None
        };
        .Some(self.abs())
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: Int16) -> Int16 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isPositive { Int16.maxValue } else { Int16.minValue }
        }
    }

    public func subtractSaturating(other: Int16) -> Int16 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isNegative { Int16.maxValue } else { Int16.minValue }
        }
    }

    public func multiplySaturating(other: Int16) -> Int16 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => {
                // Determine sign of result
                let sameSign = (self.isNegative == other.isNegative);
                if sameSign { Int16.maxValue } else { Int16.minValue }
            }
        }
    }

    public func negateSaturating() -> Int16 {
        if self == Int16.minValue {
            Int16.maxValue
        } else {
            self.negate()
        }
    }

    public func absSaturating() -> Int16 {
        if self == Int16.minValue {
            Int16.maxValue
        } else {
            self.abs()
        }
    }


    // ========================================================================
    // ARITHMETIC (Extended)
    // ========================================================================

    public func pow(exponent: Int64) -> Int16 {
        if exponent < 0 {
            return Int16.zero
        };
        if exponent == 0 {
            return Int16.one
        };
        var result = Int16.one;
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

    public func gcd(other: Int16) -> Int16 {
        var a = self.abs();
        var b = other.abs();
        while b != Int16.zero {
            let t = b;
            b = a.modulo(b);
            a = t
        };
        a
    }

    public func lcm(other: Int16) -> Int16 {
        if self == Int16.zero or other == Int16.zero {
            return Int16.zero
        };
        let g = self.gcd(other);
        self.abs().divide(g).multiply(other.abs())
    }

    // ========================================================================
    // CLAMPING
    // ========================================================================

    public func clamp(min: Int16, max: Int16) -> Int16 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    public func bitwiseAnd(other: Int16) -> Int16 { Int16(raw: lang.i16_and(self.raw, other.raw)) }
    public func bitwiseOr(other: Int16) -> Int16 { Int16(raw: lang.i16_or(self.raw, other.raw)) }
    public func bitwiseXor(other: Int16) -> Int16 { Int16(raw: lang.i16_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> Int16 { Int16(raw: lang.i16_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> Int16 { Int16(raw: lang.i16_shl(self.raw, lang.cast_i64_i16(count))) }
    public func shiftRight(by count: lang.i64) -> Int16 { Int16(raw: lang.i16_signed_shr(self.raw, lang.cast_i64_i16(count))) }

    public func rotateLeft(by count: Int64) -> Int16 {
        let bits: Int64 = 16;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    public func rotateRight(by count: Int64) -> Int16 {
        let bits: Int64 = 16;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    public mutating func addAssign(other: Int16) { self = self.add(other) }
    public mutating func subtractAssign(other: Int16) { self = self.subtract(other) }
    public mutating func multiplyAssign(other: Int16) { self = self.multiply(other) }
    public mutating func divideAssign(other: Int16) { self = self.divide(other) }
    public mutating func modAssign(other: Int16) { self = self.modulo(other) }
    public mutating func bitwiseAndAssign(other: Int16) { self = self.bitwiseAnd(other) }
    public mutating func bitwiseOrAssign(other: Int16) { self = self.bitwiseOr(other) }
    public mutating func bitwiseXorAssign(other: Int16) { self = self.bitwiseXor(other) }
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
    // public static func fromBytes(bytes: Array[UInt8]) -> Int16?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> Int16?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> Int16?

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> Int16?
    // public static func parse(string: String, radix: Int64) -> Int16?

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format() -> String {
        if self == Int16.zero {
            return "0"
        }

        var result = String();
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        let ten: Int16 = 10;
        while n != Int16.zero {
            let digit: Int16 = n % ten;
            let charCode: Int64 = Int64(from: digit) + 48;
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

