// Int8 - 8-bit signed integer
// Generated from integer.ks.template (docs synced from .ks.interface) - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable, Hash, Hasher,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    AddAssign, SubtractAssign, MultiplyAssign, DivideAssign, ModuloAssign,
    BitwiseAndAssign, BitwiseOrAssign, BitwiseXorAssign, LeftShiftAssign, RightShiftAssign,
    ExpressibleByIntLiteral, Convertible, Defaultable,
    RangeConstructible, ClosedRangeConstructible, Range, ClosedRange
)
import std.text.(String, Formattable, FormatOptions)
import std.memory.(Slice, Pointer)
import std.num.(UInt8, Int64, UInt64)

/// An 8-bit signed integer type.
/// Supports arithmetic, bitwise, comparison, and formatting operations.
public struct Int8:
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
    Defaultable,
    FFISafe,
    RangeConstructible,
    ClosedRangeConstructible,
    Convertible[Int16],
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    /// The underlying raw value.
    ///
    /// Direct access to the primitive `lang.i8` type. Useful for FFI
    /// or low-level operations.
    public var raw: lang.i8

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    /// The zero value (0).
    public static var zero: Int8 { Int8(intLiteral: 0) }

    /// The one value (1).
    public static var one: Int8 { Int8(intLiteral: 1) }

    /// The minimum representable value.
    /// This is -2^7 (-128).
    public static var minValue: Int8 { Int8(raw: lang.i8_shl(1, 7)) }

    /// The maximum representable value.
    /// This is 2^7 - 1 (127).
    public static var maxValue: Int8 { Int8(intLiteral: 127) }

    /// The number of bits in this integer type (8).
    public static var bitWidth: Int64 { Int64(intLiteral: 8) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates a Int8 from an integer literal.
    ///
    /// This initializer is called implicitly when using integer literals.
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i8(value)
    }

    /// Creates a Int8 with the default value (zero).
    public init() {
        self.init(intLiteral: 0)
    }

    /// Creates a Int8 from a raw `lang.i8` value.
    init(raw value: lang.i8) {
        self.raw = value
    }

    public init(from other: Int16) { self.raw = lang.cast_i16_i8(other.raw) }
    public init(from other: Int32) { self.raw = lang.cast_i32_i8(other.raw) }
    public init(from other: Int64) { self.raw = lang.cast_i64_i8(other.raw) }
    public init(from other: UInt8) { self.raw = other.raw }
    public init(from other: UInt16) { self.raw = lang.cast_u16_i8(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_u32_i8(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_u64_i8(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: Int8 { get {
        if Bool(boolLiteral: lang.i8_signed_lt(self.raw, 0)) { Int8(intLiteral: lang.i64_neg(1)) }
        else if Bool(boolLiteral: lang.i8_eq(self.raw, 0)) { Int8.zero }
        else { Int8.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i8_signed_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        Bool(boolLiteral: lang.i8_signed_lt(self.raw, 0))
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i8_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    /// Returns true if this value is a power of two.
    ///
    /// Zero and negative numbers are not powers of two.
    ///
    /// Example:
    ///     (1).isPowerOfTwo   // true  (2^0)
    ///     (4).isPowerOfTwo   // true  (2^2)
    ///     (3).isPowerOfTwo   // false
    ///     (0).isPowerOfTwo   // false
    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i8_signed_lt(self.raw, 1)) { false }
        else { Bool(boolLiteral: lang.i8_eq(lang.i8_and(self.raw, lang.i8_sub(self.raw, 1)), 0)) }
    }}

    /// Returns the number of 1 bits in the binary representation.
    ///
    /// Also known as "population count" or "Hamming weight".
    ///
    /// Example:
    ///     (0b1010).countOnes  // 2
    ///     (0b1111).countOnes  // 4
    ///     (0).countOnes       // 0
    public var countOnes: Int64 { get {
        Int64(raw: lang.cast_i8_i64(lang.i8_popcount(self.raw)))
    }}

    /// Returns the number of 0 bits in the binary representation.
    ///
    /// Equal to `8 - countOnes`.
    public var countZeros: Int64 { get {
        Int64(intLiteral: 8) - self.countOnes
    }}

    /// Returns the number of leading zeros in the binary representation.
    ///
    /// Example:
    ///     (1).leadingZeros   // 8 - 1
    ///     (0).leadingZeros   // 8
    public var leadingZeros: Int64 { get {
        Int64(raw: lang.cast_i8_i64(lang.i8_clz(self.raw)))
    }}

    /// Returns the number of trailing zeros in the binary representation.
    ///
    /// Useful for finding the largest power of 2 that divides this number.
    public var trailingZeros: Int64 { get {
        Int64(raw: lang.cast_i8_i64(lang.i8_ctz(self.raw)))
    }}

    /// Returns the value with its bytes in reversed order.
    ///
    /// Useful for converting between big-endian and little-endian byte orders.
    public var byteSwapped: Int8 { get {
        self
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Compares two values for equality.
    ///
    /// Example:
    ///     (42).equals(other: 42)  // true
    ///     42 == 42                // true (operator form)
    public func equals(other: Int8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.raw, other.raw))
    }

    /// Pattern matching support. Equivalent to `equals`.
    public func matches(other: Int8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.raw, other.raw))
    }

    /// Compares this value to another, returning an Ordering.
    ///
    /// Returns `.Less` if self < other, `.Greater` if self > other,
    /// or `.Equal` if they are equal.
    public func compare(other: Int8) -> Ordering {
        if Bool(boolLiteral: lang.i8_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i8_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Returns the next value (self + 1).
    public func successor() -> Int8 { self.add(Int8.one) }

    /// Returns the previous value (self - 1).
    public func predecessor() -> Int8 { self.subtract(Int8.one) }

    /// Creates an exclusive range from self to end (self..<end).
    public func exclusiveRange(to end: Int8) -> Range[Int8] {
        Range[Int8](self, end)
    }

    /// Creates an inclusive range from self to end (self..=end).
    public func inclusiveRange(to end: Int8) -> ClosedRange[Int8] {
        ClosedRange[Int8](self, end)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Hashes this value into the given hasher.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[Int8]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = Int8
    type Subtractable.Output = Int8
    type Multipliable.Output = Int8
    type Divisible.Output = Int8
    type Modulo.Output = Int8
    type Negatable.Output = Int8
    type BitwiseAnd.Output = Int8
    type BitwiseOr.Output = Int8
    type BitwiseXor.Output = Int8
    type BitwiseNot.Output = Int8
    type LeftShift.Output = Int8
    type RightShift.Output = Int8
    type RangeConstructible.Output = Range[Int8]
    type ClosedRangeConstructible.Output = ClosedRange[Int8]

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    /// Adds two values. Wraps on overflow.
    public func add(other: Int8) -> Int8 { Int8(raw: lang.i8_add(self.raw, other.raw)) }

    /// Subtracts two values. Wraps on overflow.
    public func subtract(other: Int8) -> Int8 { Int8(raw: lang.i8_sub(self.raw, other.raw)) }

    /// Multiplies two values. Wraps on overflow.
    public func multiply(other: Int8) -> Int8 { Int8(raw: lang.i8_mul(self.raw, other.raw)) }

    /// Divides two values (integer division, truncates toward zero).
    public func divide(other: Int8) -> Int8 { Int8(raw: lang.i8_signed_div(self.raw, other.raw)) }

    /// Returns the remainder of division (self % other).
    public func modulo(other: Int8) -> Int8 { Int8(raw: lang.i8_signed_rem(self.raw, other.raw)) }

    public func negate() -> Int8 { Int8(raw: lang.i8_neg(self.raw)) }
    public func abs() -> Int8 { if Bool(boolLiteral: lang.i8_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: Int8) -> Int8? {
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

    public func subtractChecked(other: Int8) -> Int8? {
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

    public func multiplyChecked(other: Int8) -> Int8? {
        if other == Int8.zero {
            return .Some(Int8.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: Int8) -> Int8? {
        if other == Int8.zero {
            return .None
        };
        // Check for minValue / -1 overflow
        if self == Int8.minValue and other == Int8(intLiteral: lang.i64_neg(1)) {
            return .None
        };
        .Some(self.divide(other))
    }

    public func negateChecked() -> Int8? {
        if self == Int8.minValue {
            return .None
        };
        .Some(self.negate())
    }

    public func absChecked() -> Int8? {
        if self == Int8.minValue {
            return .None
        };
        .Some(self.abs())
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: Int8) -> Int8 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isPositive { Int8.maxValue } else { Int8.minValue }
        }
    }

    public func subtractSaturating(other: Int8) -> Int8 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isNegative { Int8.maxValue } else { Int8.minValue }
        }
    }

    public func multiplySaturating(other: Int8) -> Int8 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => {
                // Determine sign of result
                let sameSign = (self.isNegative == other.isNegative);
                if sameSign { Int8.maxValue } else { Int8.minValue }
            }
        }
    }

    public func negateSaturating() -> Int8 {
        if self == Int8.minValue {
            Int8.maxValue
        } else {
            self.negate()
        }
    }

    public func absSaturating() -> Int8 {
        if self == Int8.minValue {
            Int8.maxValue
        } else {
            self.abs()
        }
    }


    // ========================================================================
    // ARITHMETIC (Extended)
    // ========================================================================

    /// Raises this value to the given power.
    ///
    /// Uses binary exponentiation for efficiency. Negative exponents return 0
    /// (integer division truncation).
    ///
    /// Example:
    ///     (2).pow(10)  // 1024
    ///     (3).pow(4)   // 81
    public func pow(exponent: Int64) -> Int8 {
        if exponent < 0 {
            return Int8.zero
        };
        if exponent == 0 {
            return Int8.one
        };
        var result = Int8.one;
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

    /// Returns the greatest common divisor of two values.
    ///
    /// Uses the Euclidean algorithm.
    ///
    /// Example:
    ///     (12).gcd(8)  // 4
    ///     (17).gcd(5)  // 1 (coprime)
    public func gcd(other: Int8) -> Int8 {
        var a = self.abs();
        var b = other.abs();
        while b != Int8.zero {
            let t = b;
            b = a.modulo(b);
            a = t
        };
        a
    }

    /// Returns the least common multiple of two values.
    ///
    /// Example:
    ///     (4).lcm(6)   // 12
    ///     (3).lcm(5)   // 15
    public func lcm(other: Int8) -> Int8 {
        if self == Int8.zero or other == Int8.zero {
            return Int8.zero
        };
        let g = self.gcd(other);
        self.abs().divide(g).multiply(other.abs())
    }

    // ========================================================================
    // CLAMPING
    // ========================================================================

    /// Clamps this value to the given range.
    ///
    /// Returns `min` if self < min, `max` if self > max, otherwise self.
    ///
    /// Example:
    ///     (5).clamp(min: 0, max: 10)   // 5
    ///     (-5).clamp(min: 0, max: 10)  // 0
    ///     (15).clamp(min: 0, max: 10)  // 10
    public func clamp(min: Int8, max: Int8) -> Int8 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    /// Bitwise AND. Example: `0b1010 & 0b1100 = 0b1000`
    public func bitwiseAnd(other: Int8) -> Int8 { Int8(raw: lang.i8_and(self.raw, other.raw)) }

    /// Bitwise OR. Example: `0b1010 | 0b1100 = 0b1110`
    public func bitwiseOr(other: Int8) -> Int8 { Int8(raw: lang.i8_or(self.raw, other.raw)) }

    /// Bitwise XOR. Example: `0b1010 ^ 0b1100 = 0b0110`
    public func bitwiseXor(other: Int8) -> Int8 { Int8(raw: lang.i8_xor(self.raw, other.raw)) }

    /// Bitwise NOT (complement). Flips all bits.
    public func bitwiseNot() -> Int8 { Int8(raw: lang.i8_not(self.raw)) }

    /// Left shift. Example: `1 << 4 = 16`
    public func shiftLeft(by count: lang.i64) -> Int8 { Int8(raw: lang.i8_shl(self.raw, lang.cast_i64_i8(count))) }

    /// Right shift (arithmetic for signed, logical for unsigned).
    public func shiftRight(by count: lang.i64) -> Int8 { Int8(raw: lang.i8_signed_shr(self.raw, lang.cast_i64_i8(count))) }

    /// Rotates bits left by the given count.
    public func rotateLeft(by count: Int64) -> Int8 {
        let bits: Int64 = 8;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    /// Rotates bits right by the given count.
    public func rotateRight(by count: Int64) -> Int8 {
        let bits: Int64 = 8;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    /// `self += other`
    public mutating func addAssign(other: Int8) { self = self.add(other) }
    /// `self -= other`
    public mutating func subtractAssign(other: Int8) { self = self.subtract(other) }
    /// `self *= other`
    public mutating func multiplyAssign(other: Int8) { self = self.multiply(other) }
    /// `self /= other`
    public mutating func divideAssign(other: Int8) { self = self.divide(other) }
    /// `self %= other`
    public mutating func modAssign(other: Int8) { self = self.modulo(other) }
    /// `self &= other`
    public mutating func bitwiseAndAssign(other: Int8) { self = self.bitwiseAnd(other) }
    /// `self |= other`
    public mutating func bitwiseOrAssign(other: Int8) { self = self.bitwiseOr(other) }
    /// `self ^= other`
    public mutating func bitwiseXorAssign(other: Int8) { self = self.bitwiseXor(other) }
    /// `self <<= count`
    public mutating func shiftLeftAssign(by count: lang.i64) { self = self.shiftLeft(by: count) }
    /// `self >>= count`
    public mutating func shiftRightAssign(by count: lang.i64) { self = self.shiftRight(by: count) }

    // ========================================================================
    // BYTE CONVERSION
    // ========================================================================

    // TODO: implement byte conversion methods
    // These require Array from std.collections which creates circular import issues
    // public func toBytes() -> Array[UInt8]
    // public func toBytesBigEndian() -> Array[UInt8]
    // public func toBytesLittleEndian() -> Array[UInt8]
    // public static func fromBytes(bytes: Array[UInt8]) -> Int8?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> Int8?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> Int8?

    // ========================================================================
    // PARSING
    // ========================================================================

    public static func parse(string: String) -> Int8? {
        let len = string.byteCount;
        if len == 0 {
            return .None
        }

        var index: Int64 = 0;
        var isNegative = false;

        // Check for sign
        let firstByte: UInt8 = string.byteAtUnchecked(0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {  // '-'
            isNegative = true;
            index = 1
        } else if firstByteVal == 43 {  // '+'
            index = 1
        }

        // Must have at least one digit
        if index >= len {
            return .None
        }

        // Parse digits using Int64 for accumulation
        var result: Int64 = 0;
        let maxBeforeMultiply: Int64 = 922337203685477580;  // Int64.maxValue / 10

        while index < len {
            let byte: UInt8 = string.byteAtUnchecked(index);
            let byteVal = Int64(from: byte);

            // Check if digit (0-9 = 48-57)
            if byteVal < 48 or byteVal > 57 {
                return .None
            }

            let digit = byteVal - 48;

            // Check for overflow before multiply
            if result > maxBeforeMultiply {
                return .None
            }
            result = result * 10;

            // Check for overflow before add
            if result > 9223372036854775807 - digit {
                return .None
            }
            result = result + digit;

            index = index + 1
        }

        // Apply sign and check bounds for target type
        if isNegative {
            result = result.negate();
            if result < Int64(from: Int8.minValue) {
                return .None
            }
        } else {
            if result > Int64(from: Int8.maxValue) {
                return .None
            }
        }

        .Some(Int8(from: result))
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        if self == Int8.zero {
            return "0"
        }

        var result = String();
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        let ten: Int8 = 10;
        while n != Int8.zero {
            let digit: Int8 = n % ten;
            let charCode: Int64 = Int64(from: digit) + 48;
            result.appendByte(UInt8(from: charCode));
            n = n / ten
        }

        if isNegative {
            result.appendByte(45)  // '-'
        }

        // Reverse the string
        var reversed = String();
        var i = result.byteCount - 1;
        while i >= 0 {
            reversed.appendByte(result.byteAtUnchecked(i));
            i = i - 1
        }
        reversed
    }}

