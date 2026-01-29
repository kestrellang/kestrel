// UInt64 - 64-bit unsigned integer
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
    Convertible[UInt16],
    Convertible[UInt32]
{
    public var raw: lang.i64

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    public static var zero: UInt64 { UInt64(intLiteral: 0) }
    public static var one: UInt64 { UInt64(intLiteral: 1) }
    public static var minValue: UInt64 { UInt64(intLiteral: 0) }
    public static var maxValue: UInt64 { UInt64(intLiteral: 18446744073709551615) }
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
    public init(from other: Int64) { self.raw = other.raw }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i64(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i64(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_i32_i64(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: UInt64 { get {
        if Bool(boolLiteral: lang.i64_eq(self.raw, 0)) { UInt64.zero }
        else { UInt64.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i64_unsigned_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        // Unsigned types are never negative
        false
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i64_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i64_eq(self.raw, 0)) { false }
        else { Bool(boolLiteral: lang.i64_eq(lang.i64_and(self.raw, lang.i64_sub(self.raw, 1)), 0)) }
    }}

    public var countOnes: Int64 { get {
        Int64(raw: lang.i64_popcount(self.raw))
    }}

    public var countZeros: Int64 { get {
        Int64(intLiteral: 64) - self.countOnes
    }}

    public var leadingZeros: Int64 { get {
        Int64(raw: lang.i64_clz(self.raw))
    }}

    public var trailingZeros: Int64 { get {
        Int64(raw: lang.i64_ctz(self.raw))
    }}

    public var byteSwapped: UInt64 { get {
        UInt64(raw: lang.i64_bswap(self.raw))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

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

    // ========================================================================
    // STEPPING
    // ========================================================================

    public func successor() -> UInt64 { self.add(UInt64.one) }
    public func predecessor() -> UInt64 { self.subtract(UInt64.one) }

    // ========================================================================
    // HASHING
    // ========================================================================

    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[UInt64]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

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

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    public func add(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_add(self.raw, other.raw)) }
    public func subtract(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_sub(self.raw, other.raw)) }
    public func multiply(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_mul(self.raw, other.raw)) }
    public func divide(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_div(self.raw, other.raw)) }
    public func modulo(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_rem(self.raw, other.raw)) }
    
    

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: UInt64) -> UInt64? {
        let result = self.add(other);
        // For unsigned, overflow if result < either operand
        if result < self {
            return .None
        };
        .Some(result)
    }

    public func subtractChecked(other: UInt64) -> UInt64? {
        // For unsigned, underflow if other > self
        if other > self {
            return .None
        };
        .Some(self.subtract(other))
    }

    public func multiplyChecked(other: UInt64) -> UInt64? {
        if other == UInt64.zero {
            return .Some(UInt64.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: UInt64) -> UInt64? {
        if other == UInt64.zero {
            return .None
        };
        .Some(self.divide(other))
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: UInt64) -> UInt64 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt64.maxValue
        }
    }

    public func subtractSaturating(other: UInt64) -> UInt64 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt64.zero
        }
    }

    public func multiplySaturating(other: UInt64) -> UInt64 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt64.maxValue
        }
    }


    // ========================================================================
    // ARITHMETIC (Extended)
    // ========================================================================

    public func pow(exponent: Int64) -> UInt64 {
        if exponent < 0 {
            return UInt64.zero
        };
        if exponent == 0 {
            return UInt64.one
        };
        var result = UInt64.one;
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

    public func gcd(other: UInt64) -> UInt64 {
        var a = self;
        var b = other;
        while b != UInt64.zero {
            let t = b;
            b = a.modulo(b);
            a = t
        };
        a
    }

    public func lcm(other: UInt64) -> UInt64 {
        if self == UInt64.zero or other == UInt64.zero {
            return UInt64.zero
        };
        let g = self.gcd(other);
        self.divide(g).multiply(other)
    }

    // ========================================================================
    // CLAMPING
    // ========================================================================

    public func clamp(min: UInt64, max: UInt64) -> UInt64 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    public func bitwiseAnd(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_and(self.raw, other.raw)) }
    public func bitwiseOr(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_or(self.raw, other.raw)) }
    public func bitwiseXor(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_xor(self.raw, other.raw)) }
    public func bitwiseNot() -> UInt64 { UInt64(raw: lang.i64_not(self.raw)) }
    public func shiftLeft(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_shl(self.raw, count)) }
    public func shiftRight(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_unsigned_shr(self.raw, count)) }

    public func rotateLeft(by count: Int64) -> UInt64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    public func rotateRight(by count: Int64) -> UInt64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    public mutating func addAssign(other: UInt64) { self = self.add(other) }
    public mutating func subtractAssign(other: UInt64) { self = self.subtract(other) }
    public mutating func multiplyAssign(other: UInt64) { self = self.multiply(other) }
    public mutating func divideAssign(other: UInt64) { self = self.divide(other) }
    public mutating func modAssign(other: UInt64) { self = self.modulo(other) }
    public mutating func bitwiseAndAssign(other: UInt64) { self = self.bitwiseAnd(other) }
    public mutating func bitwiseOrAssign(other: UInt64) { self = self.bitwiseOr(other) }
    public mutating func bitwiseXorAssign(other: UInt64) { self = self.bitwiseXor(other) }
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
    // public static func fromBytes(bytes: Array[UInt8]) -> UInt64?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> UInt64?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> UInt64?

    // ========================================================================
    // PARSING
    // ========================================================================

    // TODO: implement string parsing
    // public static func parse(string: String) -> UInt64?
    // public static func parse(string: String, radix: Int64) -> UInt64?

    // ========================================================================
    // FORMATTING
    // ========================================================================

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
