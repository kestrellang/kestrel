// UInt32 - 32-bit unsigned integer
// Generated from integer.ks.template - DO NOT EDIT

module std.num

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable, Formattable, Hash, Hasher,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    AddAssign, SubtractAssign, MultiplyAssign, DivideAssign, ModuloAssign,
    BitwiseAndAssign, BitwiseOrAssign, BitwiseXorAssign, LeftShiftAssign, RightShiftAssign,
    ExpressibleByIntLiteral, Convertible,
    RangeConstructible, ClosedRangeConstructible, Range, ClosedRange
)
import std.text.(String)
import std.memory.(Slice, Pointer)
import std.num.(UInt8, Int64)

/// A 32-bit unsigned integer type.
///
/// UInt32 supports arithmetic, bitwise, comparison, and formatting operations.
/// It is FFI-safe for interoperability with C code.
///
/// Arithmetic operations wrap on overflow (two's complement) by default.
/// Use checked methods for overflow detection or saturating methods for
/// clamping behavior.
///
/// Example:
///     let a: UInt32 = 100
///     let b = a + 50
///     let c = a * 2
public struct UInt32:
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
    RangeConstructible,
    ClosedRangeConstructible,
    Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt64]
{
    /// The underlying raw value.
    ///
    /// Direct access to the primitive `lang.i32` type. Useful for FFI
    /// or low-level operations.
    public var raw: lang.i32

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    /// The zero value (0).
    public static var zero: UInt32 { UInt32(intLiteral: 0) }

    /// The one value (1).
    public static var one: UInt32 { UInt32(intLiteral: 1) }

    /// The minimum representable value.
    /// This is always 0 for unsigned types.
    public static var minValue: UInt32 { UInt32(intLiteral: 0) }

    /// The maximum representable value.
    /// This is 2^32 - 1 (4_294_967_295).
    public static var maxValue: UInt32 { UInt32(intLiteral: 4294967295) }

    /// The number of bits in this integer type (32).
    public static var bitWidth: Int64 { Int64(intLiteral: 32) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates a UInt32 from an integer literal.
    ///
    /// This initializer is called implicitly when using integer literals.
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i32(value)
    }

    /// Creates a UInt32 from a raw `lang.i32` value.
    init(raw value: lang.i32) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: Int16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: Int32) { self.raw = other.raw }
    public init(from other: Int64) { self.raw = lang.cast_i64_i32(other.raw) }
    public init(from other: UInt8) { self.raw = lang.cast_i8_i32(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_i16_i32(other.raw) }
    public init(from other: UInt64) { self.raw = lang.cast_i64_i32(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    public var sign: UInt32 { get {
        if Bool(boolLiteral: lang.i32_eq(self.raw, 0)) { UInt32.zero }
        else { UInt32.one }
    }}

    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i32_unsigned_gt(self.raw, 0))
    }}

    public var isNegative: Bool { get {
        // Unsigned types are never negative
        false
    }}

    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i32_eq(self.raw, 0))
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
        if Bool(boolLiteral: lang.i32_eq(self.raw, 0)) { false }
        else { Bool(boolLiteral: lang.i32_eq(lang.i32_and(self.raw, lang.i32_sub(self.raw, 1)), 0)) }
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
        Int64(raw: lang.cast_i32_i64(lang.i32_popcount(self.raw)))
    }}

    /// Returns the number of 0 bits in the binary representation.
    ///
    /// Equal to `32 - countOnes`.
    public var countZeros: Int64 { get {
        Int64(intLiteral: 32) - self.countOnes
    }}

    /// Returns the number of leading zeros in the binary representation.
    ///
    /// Example:
    ///     (1).leadingZeros   // 32 - 1
    ///     (0).leadingZeros   // 32
    public var leadingZeros: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_clz(self.raw)))
    }}

    /// Returns the number of trailing zeros in the binary representation.
    ///
    /// Useful for finding the largest power of 2 that divides this number.
    public var trailingZeros: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_ctz(self.raw)))
    }}

    /// Returns the value with its bytes in reversed order.
    ///
    /// Useful for converting between big-endian and little-endian byte orders.
    public var byteSwapped: UInt32 { get {
        UInt32(raw: lang.i32_bswap(self.raw))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Compares two values for equality.
    ///
    /// Example:
    ///     (42).equals(other: 42)  // true
    ///     42 == 42                // true (operator form)
    public func equals(other: UInt32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    /// Pattern matching support. Equivalent to `equals`.
    public func matches(other: UInt32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    /// Compares this value to another, returning an Ordering.
    ///
    /// Returns `.Less` if self < other, `.Greater` if self > other,
    /// or `.Equal` if they are equal.
    public func compare(other: UInt32) -> Ordering {
        if Bool(boolLiteral: lang.i32_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i32_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Returns the next value (self + 1).
    public func successor() -> UInt32 { self.add(UInt32.one) }

    /// Returns the previous value (self - 1).
    public func predecessor() -> UInt32 { self.subtract(UInt32.one) }

    /// Creates an exclusive range from self to end (self..<end).
    public func exclusiveRange(to end: UInt32) -> Range[UInt32] {
        Range[UInt32](self, end)
    }

    /// Creates an inclusive range from self to end (self..=end).
    public func inclusiveRange(to end: UInt32) -> ClosedRange[UInt32] {
        ClosedRange[UInt32](self, end)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Hashes this value into the given hasher.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: lang.sizeof[UInt32]())))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = UInt32
    type Subtractable.Output = UInt32
    type Multipliable.Output = UInt32
    type Divisible.Output = UInt32
    type Modulo.Output = UInt32
    
    type BitwiseAnd.Output = UInt32
    type BitwiseOr.Output = UInt32
    type BitwiseXor.Output = UInt32
    type BitwiseNot.Output = UInt32
    type LeftShift.Output = UInt32
    type RightShift.Output = UInt32
    type RangeConstructible.Output = Range[UInt32]
    type ClosedRangeConstructible.Output = ClosedRange[UInt32]

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    /// Adds two values. Wraps on overflow.
    public func add(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_add(self.raw, other.raw)) }

    /// Subtracts two values. Wraps on overflow.
    public func subtract(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_sub(self.raw, other.raw)) }

    /// Multiplies two values. Wraps on overflow.
    public func multiply(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_mul(self.raw, other.raw)) }

    /// Divides two values (integer division, truncates toward zero).
    public func divide(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_div(self.raw, other.raw)) }

    /// Returns the remainder of division (self % other).
    public func modulo(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_rem(self.raw, other.raw)) }

    
    

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: UInt32) -> UInt32? {
        let result = self.add(other);
        // For unsigned, overflow if result < either operand
        if result < self {
            return .None
        };
        .Some(result)
    }

    public func subtractChecked(other: UInt32) -> UInt32? {
        // For unsigned, underflow if other > self
        if other > self {
            return .None
        };
        .Some(self.subtract(other))
    }

    public func multiplyChecked(other: UInt32) -> UInt32? {
        if other == UInt32.zero {
            return .Some(UInt32.zero)
        };
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {
            return .None
        };
        .Some(result)
    }

    public func divideChecked(other: UInt32) -> UInt32? {
        if other == UInt32.zero {
            return .None
        };
        .Some(self.divide(other))
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    public func addSaturating(other: UInt32) -> UInt32 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt32.maxValue
        }
    }

    public func subtractSaturating(other: UInt32) -> UInt32 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt32.zero
        }
    }

    public func multiplySaturating(other: UInt32) -> UInt32 {
        let checked = self.multiplyChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt32.maxValue
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
    public func pow(exponent: Int64) -> UInt32 {
        if exponent < 0 {
            return UInt32.zero
        };
        if exponent == 0 {
            return UInt32.one
        };
        var result = UInt32.one;
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
    public func gcd(other: UInt32) -> UInt32 {
        var a = self;
        var b = other;
        while b != UInt32.zero {
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
    public func lcm(other: UInt32) -> UInt32 {
        if self == UInt32.zero or other == UInt32.zero {
            return UInt32.zero
        };
        let g = self.gcd(other);
        self.divide(g).multiply(other)
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
    public func clamp(min: UInt32, max: UInt32) -> UInt32 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    /// Bitwise AND. Example: `0b1010 & 0b1100 = 0b1000`
    public func bitwiseAnd(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_and(self.raw, other.raw)) }

    /// Bitwise OR. Example: `0b1010 | 0b1100 = 0b1110`
    public func bitwiseOr(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_or(self.raw, other.raw)) }

    /// Bitwise XOR. Example: `0b1010 ^ 0b1100 = 0b0110`
    public func bitwiseXor(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_xor(self.raw, other.raw)) }

    /// Bitwise NOT (complement). Flips all bits.
    public func bitwiseNot() -> UInt32 { UInt32(raw: lang.i32_not(self.raw)) }

    /// Left shift. Example: `1 << 4 = 16`
    public func shiftLeft(by count: lang.i64) -> UInt32 { UInt32(raw: lang.i32_shl(self.raw, lang.cast_i64_i32(count))) }

    /// Right shift (arithmetic for signed, logical for unsigned).
    public func shiftRight(by count: lang.i64) -> UInt32 { UInt32(raw: lang.i32_unsigned_shr(self.raw, lang.cast_i64_i32(count))) }

    /// Rotates bits left by the given count.
    public func rotateLeft(by count: Int64) -> UInt32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    /// Rotates bits right by the given count.
    public func rotateRight(by count: Int64) -> UInt32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    /// `self += other`
    public mutating func addAssign(other: UInt32) { self = self.add(other) }
    /// `self -= other`
    public mutating func subtractAssign(other: UInt32) { self = self.subtract(other) }
    /// `self *= other`
    public mutating func multiplyAssign(other: UInt32) { self = self.multiply(other) }
    /// `self /= other`
    public mutating func divideAssign(other: UInt32) { self = self.divide(other) }
    /// `self %= other`
    public mutating func modAssign(other: UInt32) { self = self.modulo(other) }
    /// `self &= other`
    public mutating func bitwiseAndAssign(other: UInt32) { self = self.bitwiseAnd(other) }
    /// `self |= other`
    public mutating func bitwiseOrAssign(other: UInt32) { self = self.bitwiseOr(other) }
    /// `self ^= other`
    public mutating func bitwiseXorAssign(other: UInt32) { self = self.bitwiseXor(other) }
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
    // public static func fromBytes(bytes: Array[UInt8]) -> UInt32?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> UInt32?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> UInt32?

    // ========================================================================
    // PARSING
    // ========================================================================

    public static func parse(string: String) -> UInt32? {
        let len = string.byteCount;
        if len == 0 {
            return .None
        }

        var index: Int64 = 0;

        // Check for optional + sign
        let firstByte: UInt8 = string.byteAtUnchecked(0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 43 {  // '+'
            index = 1
        } else if firstByteVal == 45 {  // '-' not allowed for unsigned
            return .None
        }

        // Must have at least one digit
        if index >= len {
            return .None
        }

        // Parse digits using UInt64 for accumulation
        var result: UInt64 = 0;
        let maxBeforeMultiply: UInt64 = 1844674407370955161;
        let maxVal: UInt64 = UInt64(from: UInt32.maxValue);

        while index < len {
            let byte: UInt8 = string.byteAtUnchecked(index);
            let byteVal = UInt64(from: byte);

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
            if result > UInt64.maxValue - digit {
                return .None
            }
            result = result + digit;

            index = index + 1
        }

        // Check bounds for target type
        if result > maxVal {
            return .None
        }

        .Some(UInt32(from: result))
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    public func format() -> String {
        if self == UInt32.zero {
            return "0"
        }

        var result = String();
        var n = self;

        let ten: UInt32 = 10;
        while n != UInt32.zero {
            let digit: UInt32 = n % ten;
            let charCode: Int64 = Int64(from: digit) + 48;
            result.appendByte(UInt8(from: charCode));
            n = n / ten
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

