// Int32 - 32-bit signed integer
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

public struct Int32:
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
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i32

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    public static var zero: Int32 { Int32(intLiteral: 0) }
    public static var one: Int32 { Int32(intLiteral: 1) }
    public static var minValue: Int32 { Int32(intLiteral: lang.i64_neg(2147483648)) }
    public static var maxValue: Int32 { Int32(intLiteral: 2147483647) }
    public static var bitWidth: Int64 { Int64(intLiteral: 32) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i32(value)
    }

    init(raw value: lang.i32) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: Int16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i32(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: UInt32) { self.raw = other.raw }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i32(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: Int32 { get {
        if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0)) { Int32(intLiteral: lang.i64_neg(1)) }
        else if Bool(boolLiteral: lang.i32_eq(self.raw, 0)) { Int32.zero }
        else { Int32.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i32_signed_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0))
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i32_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 1)) { false }
        else { Bool(boolLiteral: lang.i32_eq(lang.i32_and(self.raw, lang.i32_sub(self.raw, 1)), 0)) }
    }}

    public var countOnes: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_popcount(self.raw)))
    }}

    public var countZeros: Int64 { get {
        Int64(intLiteral: 32) - self.countOnes
    }}

    public var leadingZeros: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_clz(self.raw)))
    }}

    public var trailingZeros: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_ctz(self.raw)))
    }}

    public var byteSwapped: Int32 { get {
        Int32(raw: lang.i32_bswap(self.raw))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    public func equals(other: Int32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    public func matches(other: Int32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    public func compare(other: Int32) -> Ordering {
        if Bool(boolLiteral: lang.i32_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i32_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    public func successor() -> Int32 { self.add(Int32.one) }
    public func predecessor() -> Int32 { self.subtract(Int32.one) }

    // ========================================================================
    // HASHING
    // ========================================================================

    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[Int32]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = Int32
    type Subtractable.Output = Int32
    type Multipliable.Output = Int32
    type Divisible.Output = Int32
    type Modulo.Output = Int32
    type Negatable.Output = Int32
    type BitwiseAnd.Output = Int32
    type BitwiseOr.Output = Int32
    type BitwiseXor.Output = Int32
    type BitwiseNot.Output = Int32
    type LeftShift.Output = Int32
    type RightShift.Output = Int32

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    public func add(other: Int32) -> Int32 { Int32(raw: lang.i32_add(self.raw, other.raw)) }
    public func subtract(other: Int32) -> Int32 { Int32(raw: lang.i32_sub(self.raw, other.raw)) }
    public func multiply(other: Int32) -> Int32 { Int32(raw: lang.i32_mul(self.raw, other.raw)) }
    public func divide(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_div(self.raw, other.raw)) }
    public func modulo(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_rem(self.raw, other.raw)) }
    public func negate() -> Int32 { Int32(raw: lang.i32_neg(self.raw)) }
    public func abs() -> Int32 { if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: Int32) -> Int32? {
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

    public func subtractChecked(other: Int32) -> Int32? {
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

    public func multiplyChecked(other: Int32) -> Int32? {
        if other == Int32.zero {
            return .Some(Int32.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: Int32) -> Int32? {
        if other == Int32.zero {
            return .None
        };
        // Check for minValue / -1 overflow
        if self == Int32.minValue and other == Int32(intLiteral: lang.i64_neg(1)) {
            return .None
        };
        .Some(self.divide(other))
    }

    public func negateChecked() -> Int32? {
        if self == Int32.minValue {
            return .None
        };
        .Some(self.negate())
    }

    public func absChecked() -> Int32? {
        if self == Int32.minValue {
            return .None
        };
        .Some(self.abs())
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: Int32) -> Int32 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isPositive { Int32.maxValue } else { Int32.minValue }
        }
    }

    public func subtractSaturating(other: Int32) -> Int32 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isNegative { Int32.maxValue } else { Int32.minValue }
        }
    }

    public func multiplySaturating(other: Int32) -> Int32 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => {
                // Determine sign of result
                let sameSign = (self.isNegative == other.isNegative);
                if sameSign { Int32.maxValue } else { Int32.minValue }
            }
        }
    }

    public func negateSaturating() -> Int32 {
        if self == Int32.minValue {
            Int32.maxValue
        } else {
            self.negate()
        }
    }

    public func absSaturating() -> Int32 {
        if self == Int32.minValue {
            Int32.maxValue
        } else {
            self.abs()
        }
    }


    // ========================================================================
    // ARITHMETIC (Extended)
    // ========================================================================

    public func pow(exponent: Int64) -> Int32 {
        if exponent < 0 {
            return Int32.zero
        };
        if exponent == 0 {
            return Int32.one
        };
        var result = Int32.one;
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

    public func gcd(other: Int32) -> Int32 {
        var a = self.abs();
        var b = other.abs();
        while b != Int32.zero {
            let t = b;
            b = a.modulo(b);
            a = t
        };
        a
    }

    public func lcm(other: Int32) -> Int32 {
        if self == Int32.zero or other == Int32.zero {
            return Int32.zero
        };
        let g = self.gcd(other);
        self.abs().divide(g).multiply(other.abs())
    }

    // ========================================================================
    // CLAMPING
    // ========================================================================

    public func clamp(min: Int32, max: Int32) -> Int32 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    public func bitwiseAnd(other: Int32) -> Int32 { Int32(raw: lang.i32_and(self.raw, other.raw)) }
    public func bitwiseOr(other: Int32) -> Int32 { Int32(raw: lang.i32_or(self.raw, other.raw)) }
    public func bitwiseXor(other: Int32) -> Int32 { Int32(raw: lang.i32_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> Int32 { Int32(raw: lang.i32_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> Int32 { Int32(raw: lang.i32_shl(self.raw, lang.cast_i64_i32(count))) }
    public func shiftRight(by count: lang.i64) -> Int32 { Int32(raw: lang.i32_signed_shr(self.raw, lang.cast_i64_i32(count))) }

    public func rotateLeft(by count: Int64) -> Int32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    public func rotateRight(by count: Int64) -> Int32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    public mutating func addAssign(other: Int32) { self = self.add(other) }
    public mutating func subtractAssign(other: Int32) { self = self.subtract(other) }
    public mutating func multiplyAssign(other: Int32) { self = self.multiply(other) }
    public mutating func divideAssign(other: Int32) { self = self.divide(other) }
    public mutating func modAssign(other: Int32) { self = self.modulo(other) }
    public mutating func bitwiseAndAssign(other: Int32) { self = self.bitwiseAnd(other) }
    public mutating func bitwiseOrAssign(other: Int32) { self = self.bitwiseOr(other) }
    public mutating func bitwiseXorAssign(other: Int32) { self = self.bitwiseXor(other) }
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
    // public static func fromBytes(bytes: Array[UInt8]) -> Int32?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> Int32?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> Int32?

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> Int32?
    // public static func parse(string: String, radix: Int64) -> Int32?

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format() -> String {
        if self == Int32.zero {
            return "0"
        }

        var result = String();
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        let ten: Int32 = 10;
        while n != Int32.zero {
            let digit: Int32 = n % ten;
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

