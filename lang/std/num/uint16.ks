// UInt16 - 16-bit unsigned integer
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

public struct UInt16:
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
    Convertible[UInt8],
    Convertible[UInt32],
    Convertible[UInt64]
{
    public var raw: lang.i16

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    public static var zero: UInt16 { UInt16(intLiteral: 0) }
    public static var one: UInt16 { UInt16(intLiteral: 1) }
    public static var minValue: UInt16 { UInt16(intLiteral: 0) }
    public static var maxValue: UInt16 { UInt16(intLiteral: 65535) }
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
    public init(from other: Int16) { self.raw = other.raw }
    public init(from other: Int32) { self.raw = lang.cast_i32_i16(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i16(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i16(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i16(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i16(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: UInt16 { get {
        if Bool(boolLiteral: lang.i16_eq(self.raw, 0)) { UInt16.zero }
        else { UInt16.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i16_unsigned_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        // Unsigned types are never negative
        false
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i16_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i16_eq(self.raw, 0)) { false }
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
            n = lang.i16_unsigned_shr(n, 1);
            i = i + 1
        };
        count
    }}

    public var countZeros: Int64 { get {
        Int64(intLiteral: 16) - self.countOnes
    }}

    // TODO: requires lang.i16_clz intrinsic
    public var leadingZeros: Int64 { get {
        if self == UInt16.zero {
            return Int64(intLiteral: 16)
        };
        var count: Int64 = 0;
        var n = self.raw;
        var i: Int64 = 16 - 1;
        while i >= 0 {
            let bit = lang.i16_and(lang.i16_unsigned_shr(n, lang.cast_i64_i16(i.raw)), 1);
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
        if self == UInt16.zero {
            return Int64(intLiteral: 16)
        };
        var count: Int64 = 0;
        var n = self.raw;
        while Bool(boolLiteral: lang.i16_eq(lang.i16_and(n, 1), 0)) {
            count = count + 1;
            n = lang.i16_unsigned_shr(n, 1)
        };
        count
    }}

    // TODO: requires lang.i16_bswap intrinsic
    public var byteSwapped: UInt16 { get {
        UInt16(raw: lang.i16_or(
            lang.i16_shl(lang.i16_and(self.raw, 255), 8),
            lang.i16_and(lang.i16_unsigned_shr(self.raw, 8), 255)
        ))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    public func equals(other: UInt16) -> Bool {
        Bool(boolLiteral: lang.i16_eq(self.raw, other.raw))
    }

    public func matches(other: UInt16) -> Bool {
        Bool(boolLiteral: lang.i16_eq(self.raw, other.raw))
    }

    public func compare(other: UInt16) -> Ordering {
        if Bool(boolLiteral: lang.i16_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i16_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    public func successor() -> UInt16 { self.add(UInt16.one) }
    public func predecessor() -> UInt16 { self.subtract(UInt16.one) }

    // ========================================================================
    // HASHING
    // ========================================================================

    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[UInt16]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = UInt16
    type Subtractable.Output = UInt16
    type Multipliable.Output = UInt16
    type Divisible.Output = UInt16
    type Modulo.Output = UInt16
    
    type BitwiseAnd.Output = UInt16
    type BitwiseOr.Output = UInt16
    type BitwiseXor.Output = UInt16
    type BitwiseNot.Output = UInt16
    type LeftShift.Output = UInt16
    type RightShift.Output = UInt16

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    public func add(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_add(self.raw, other.raw)) }
    public func subtract(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_sub(self.raw, other.raw)) }
    public func multiply(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_mul(self.raw, other.raw)) }
    public func divide(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_unsigned_div(self.raw, other.raw)) }
    public func modulo(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_unsigned_rem(self.raw, other.raw)) }
    
    

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: UInt16) -> UInt16? {
        let result = self.add(other);
        // For unsigned, overflow if result < either operand
        if result < self {
            return .None
        };
        .Some(result)
    }

    public func subtractChecked(other: UInt16) -> UInt16? {
        // For unsigned, underflow if other > self
        if other > self {
            return .None
        };
        .Some(self.subtract(other))
    }

    public func multiplyChecked(other: UInt16) -> UInt16? {
        if other == UInt16.zero {
            return .Some(UInt16.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: UInt16) -> UInt16? {
        if other == UInt16.zero {
            return .None
        };
        .Some(self.divide(other))
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: UInt16) -> UInt16 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt16.maxValue
        }
    }

    public func subtractSaturating(other: UInt16) -> UInt16 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt16.zero
        }
    }

    public func multiplySaturating(other: UInt16) -> UInt16 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt16.maxValue
        }
    }


    // ========================================================================
    // ARITHMETIC (Extended)
    // ========================================================================

    public func pow(exponent: Int64) -> UInt16 {
        if exponent < 0 {
            return UInt16.zero
        };
        if exponent == 0 {
            return UInt16.one
        };
        var result = UInt16.one;
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

    public func gcd(other: UInt16) -> UInt16 {
        var a = self;
        var b = other;
        while b != UInt16.zero {
            let t = b;
            b = a.modulo(b);
            a = t
        };
        a
    }

    public func lcm(other: UInt16) -> UInt16 {
        if self == UInt16.zero or other == UInt16.zero {
            return UInt16.zero
        };
        let g = self.gcd(other);
        self.divide(g).multiply(other)
    }

    // ========================================================================
    // CLAMPING
    // ========================================================================

    public func clamp(min: UInt16, max: UInt16) -> UInt16 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    public func bitwiseAnd(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_and(self.raw, other.raw)) }
    public func bitwiseOr(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_or(self.raw, other.raw)) }
    public func bitwiseXor(other: UInt16) -> UInt16 { UInt16(raw: lang.i16_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> UInt16 { UInt16(raw: lang.i16_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> UInt16 { UInt16(raw: lang.i16_shl(self.raw, lang.cast_i64_i16(count))) }
    public func shiftRight(by count: lang.i64) -> UInt16 { UInt16(raw: lang.i16_unsigned_shr(self.raw, lang.cast_i64_i16(count))) }

    public func rotateLeft(by count: Int64) -> UInt16 {
        let bits: Int64 = 16;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    public func rotateRight(by count: Int64) -> UInt16 {
        let bits: Int64 = 16;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    public mutating func addAssign(other: UInt16) { self = self.add(other) }
    public mutating func subtractAssign(other: UInt16) { self = self.subtract(other) }
    public mutating func multiplyAssign(other: UInt16) { self = self.multiply(other) }
    public mutating func divideAssign(other: UInt16) { self = self.divide(other) }
    public mutating func modAssign(other: UInt16) { self = self.modulo(other) }
    public mutating func bitwiseAndAssign(other: UInt16) { self = self.bitwiseAnd(other) }
    public mutating func bitwiseOrAssign(other: UInt16) { self = self.bitwiseOr(other) }
    public mutating func bitwiseXorAssign(other: UInt16) { self = self.bitwiseXor(other) }
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
    // public static func fromBytes(bytes: Array[UInt8]) -> UInt16?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> UInt16?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> UInt16?

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> UInt16?
    // public static func parse(string: String, radix: Int64) -> UInt16?

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format() -> String {
        if self == UInt16.zero {
            return "0"
        }

        var result = String();
        var n = self;

        let ten: UInt16 = 10;
        while n != UInt16.zero {
            let digit: UInt16 = n % ten;
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

