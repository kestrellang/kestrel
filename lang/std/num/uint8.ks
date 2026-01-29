// UInt8 - 8-bit unsigned integer
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

public struct UInt8:
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
    Convertible[Int64],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i8

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    public static var zero: UInt8 { UInt8(intLiteral: 0) }
    public static var one: UInt8 { UInt8(intLiteral: 1) }
    public static var minValue: UInt8 { UInt8(intLiteral: 0) }
    public static var maxValue: UInt8 { UInt8(intLiteral: 255) }
    public static var bitWidth: Int64 { Int64(intLiteral: 8) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i8(value)
    }

    init(raw value: lang.i8) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = other.raw }
    public init(from other: Int16) { self.raw = lang.cast_i16_i8(other.raw) }
    public init(from other: Int32) { self.raw = lang.cast_i32_i8(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i8(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i8(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i8(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i8(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: UInt8 { get {
        if Bool(boolLiteral: lang.i8_eq(self.raw, 0)) { UInt8.zero }
        else { UInt8.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i8_unsigned_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        // Unsigned types are never negative
        false
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i8_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i8_eq(self.raw, 0)) { false }
        else { Bool(boolLiteral: lang.i8_eq(lang.i8_and(self.raw, lang.i8_sub(self.raw, 1)), 0)) }
    }}

    // TODO: requires lang.i8_popcount intrinsic
    public var countOnes: Int64 { get {
        // Stub implementation - counts bits manually
        var count: Int64 = 0;
        var n = self.raw;
        var i: Int64 = 0;
        while i < 8 {
            if not Bool(boolLiteral: lang.i8_eq(lang.i8_and(n, 1), 0)) {
                count = count + 1
            };
            n = lang.i8_unsigned_shr(n, 1);
            i = i + 1
        };
        count
    }}

    public var countZeros: Int64 { get {
        Int64(intLiteral: 8) - self.countOnes
    }}

    // TODO: requires lang.i8_clz intrinsic
    public var leadingZeros: Int64 { get {
        if self == UInt8.zero {
            return Int64(intLiteral: 8)
        };
        var count: Int64 = 0;
        var n = self.raw;
        var i: Int64 = 8 - 1;
        while i >= 0 {
            let bit = lang.i8_and(lang.i8_unsigned_shr(n, lang.cast_i64_i8(i.raw)), 1);
            if not Bool(boolLiteral: lang.i8_eq(bit, 0)) {
                return count
            };
            count = count + 1;
            i = i - 1
        };
        count
    }}

    // TODO: requires lang.i8_ctz intrinsic
    public var trailingZeros: Int64 { get {
        if self == UInt8.zero {
            return Int64(intLiteral: 8)
        };
        var count: Int64 = 0;
        var n = self.raw;
        while Bool(boolLiteral: lang.i8_eq(lang.i8_and(n, 1), 0)) {
            count = count + 1;
            n = lang.i8_unsigned_shr(n, 1)
        };
        count
    }}

    // TODO: requires lang.i8_bswap intrinsic
    public var byteSwapped: UInt8 { get {
        self
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    public func equals(other: UInt8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.raw, other.raw))
    }

    public func matches(other: UInt8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.raw, other.raw))
    }

    public func compare(other: UInt8) -> Ordering {
        if Bool(boolLiteral: lang.i8_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i8_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    public func successor() -> UInt8 { self.add(UInt8.one) }
    public func predecessor() -> UInt8 { self.subtract(UInt8.one) }

    // ========================================================================
    // HASHING
    // ========================================================================

    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[UInt8]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = UInt8
    type Subtractable.Output = UInt8
    type Multipliable.Output = UInt8
    type Divisible.Output = UInt8
    type Modulo.Output = UInt8
    
    type BitwiseAnd.Output = UInt8
    type BitwiseOr.Output = UInt8
    type BitwiseXor.Output = UInt8
    type BitwiseNot.Output = UInt8
    type LeftShift.Output = UInt8
    type RightShift.Output = UInt8

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    public func add(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_add(self.raw, other.raw)) }
    public func subtract(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_sub(self.raw, other.raw)) }
    public func multiply(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_mul(self.raw, other.raw)) }
    public func divide(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_unsigned_div(self.raw, other.raw)) }
    public func modulo(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_unsigned_rem(self.raw, other.raw)) }
    
    

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: UInt8) -> UInt8? {
        let result = self.add(other);
        // For unsigned, overflow if result < either operand
        if result < self {
            return .None
        };
        .Some(result)
    }

    public func subtractChecked(other: UInt8) -> UInt8? {
        // For unsigned, underflow if other > self
        if other > self {
            return .None
        };
        .Some(self.subtract(other))
    }

    public func multiplyChecked(other: UInt8) -> UInt8? {
        if other == UInt8.zero {
            return .Some(UInt8.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: UInt8) -> UInt8? {
        if other == UInt8.zero {
            return .None
        };
        .Some(self.divide(other))
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: UInt8) -> UInt8 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt8.maxValue
        }
    }

    public func subtractSaturating(other: UInt8) -> UInt8 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt8.zero
        }
    }

    public func multiplySaturating(other: UInt8) -> UInt8 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt8.maxValue
        }
    }


    // ========================================================================
    // ARITHMETIC (Extended)
    // ========================================================================

    public func pow(exponent: Int64) -> UInt8 {
        if exponent < 0 {
            return UInt8.zero
        };
        if exponent == 0 {
            return UInt8.one
        };
        var result = UInt8.one;
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

    public func gcd(other: UInt8) -> UInt8 {
        var a = self;
        var b = other;
        while b != UInt8.zero {
            let t = b;
            b = a.modulo(b);
            a = t
        };
        a
    }

    public func lcm(other: UInt8) -> UInt8 {
        if self == UInt8.zero or other == UInt8.zero {
            return UInt8.zero
        };
        let g = self.gcd(other);
        self.divide(g).multiply(other)
    }

    // ========================================================================
    // CLAMPING
    // ========================================================================

    public func clamp(min: UInt8, max: UInt8) -> UInt8 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    public func bitwiseAnd(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_and(self.raw, other.raw)) }
    public func bitwiseOr(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_or(self.raw, other.raw)) }
    public func bitwiseXor(other: UInt8) -> UInt8 { UInt8(raw: lang.i8_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> UInt8 { UInt8(raw: lang.i8_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> UInt8 { UInt8(raw: lang.i8_shl(self.raw, lang.cast_i64_i8(count))) }
    public func shiftRight(by count: lang.i64) -> UInt8 { UInt8(raw: lang.i8_unsigned_shr(self.raw, lang.cast_i64_i8(count))) }

    public func rotateLeft(by count: Int64) -> UInt8 {
        let bits: Int64 = 8;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    public func rotateRight(by count: Int64) -> UInt8 {
        let bits: Int64 = 8;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    public mutating func addAssign(other: UInt8) { self = self.add(other) }
    public mutating func subtractAssign(other: UInt8) { self = self.subtract(other) }
    public mutating func multiplyAssign(other: UInt8) { self = self.multiply(other) }
    public mutating func divideAssign(other: UInt8) { self = self.divide(other) }
    public mutating func modAssign(other: UInt8) { self = self.modulo(other) }
    public mutating func bitwiseAndAssign(other: UInt8) { self = self.bitwiseAnd(other) }
    public mutating func bitwiseOrAssign(other: UInt8) { self = self.bitwiseOr(other) }
    public mutating func bitwiseXorAssign(other: UInt8) { self = self.bitwiseXor(other) }
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
    // public static func fromBytes(bytes: Array[UInt8]) -> UInt8?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> UInt8?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> UInt8?

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> UInt8?
    // public static func parse(string: String, radix: Int64) -> UInt8?

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format() -> String {
        if self == UInt8.zero {
            return "0"
        }

        var result = String();
        var n = self;

        let ten: UInt8 = 10;
        while n != UInt8.zero {
            let digit: UInt8 = n % ten;
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

