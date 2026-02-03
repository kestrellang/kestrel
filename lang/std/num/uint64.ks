// UInt64 - 64-bit unsigned integer
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

/// A 64-bit unsigned integer type.
/// Supports arithmetic, bitwise, comparison, and formatting operations.
/// FFI-safe for interoperability with C code.
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
    Defaultable,
    FFISafe,
    RangeConstructible,
    ClosedRangeConstructible,
    Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32]
{
    /// The underlying raw value.
    ///
    /// Direct access to the primitive `lang.i64` type. Useful for FFI
    /// or low-level operations.
    public var raw: lang.i64

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    /// The zero value (0).
    public static var zero: UInt64 { UInt64(intLiteral: 0) }

    /// The one value (1).
    public static var one: UInt64 { UInt64(intLiteral: 1) }

    /// The minimum representable value.
    /// This is always 0 for unsigned types.
    public static var minValue: UInt64 { UInt64(intLiteral: 0) }

    /// The maximum representable value.
    /// This is 2^64 - 1 (18_446_744_073_709_551_615).
    public static var maxValue: UInt64 { UInt64(intLiteral: 18446744073709551615) }

    /// The number of bits in this integer type (64).
    public static var bitWidth: Int64 { Int64(intLiteral: 64) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates a UInt64 from an integer literal.
    ///
    /// This initializer is called implicitly when using integer literals.
    public init(intLiteral value: lang.i64) {
        self.raw = value
    }

    /// Creates a UInt64 with the default value (zero).
    public init() {
        self.init(intLiteral: 0)
    }

    /// Creates a UInt64 from a raw `lang.i64` value.
    init(raw value: lang.i64) {
        self.raw = value
    }

    public init(from other: Int8) { self.raw = lang.cast_i8_i64(other.raw) }
    public init(from other: Int16) { self.raw = lang.cast_i16_i64(other.raw) }
    public init(from other: Int32) { self.raw = lang.cast_i32_i64(other.raw) }
    public init(from other: Int64) { self.raw = other.raw }
    public init(from other: UInt8) { self.raw = lang.cast_u8_i64(other.raw) }
    public init(from other: UInt16) { self.raw = lang.cast_u16_i64(other.raw) }
    public init(from other: UInt32) { self.raw = lang.cast_u32_i64(other.raw) }

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
        if Bool(boolLiteral: lang.i64_eq(self.raw, 0)) { false }
        else { Bool(boolLiteral: lang.i64_eq(lang.i64_and(self.raw, lang.i64_sub(self.raw, 1)), 0)) }
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
        Int64(raw: lang.i64_popcount(self.raw))
    }}

    /// Returns the number of 0 bits in the binary representation.
    ///
    /// Equal to `64 - countOnes`.
    public var countZeros: Int64 { get {
        Int64(intLiteral: 64) - self.countOnes
    }}

    /// Returns the number of leading zeros in the binary representation.
    ///
    /// Example:
    ///     (1).leadingZeros   // 64 - 1
    ///     (0).leadingZeros   // 64
    public var leadingZeros: Int64 { get {
        Int64(raw: lang.i64_clz(self.raw))
    }}

    /// Returns the number of trailing zeros in the binary representation.
    ///
    /// Useful for finding the largest power of 2 that divides this number.
    public var trailingZeros: Int64 { get {
        Int64(raw: lang.i64_ctz(self.raw))
    }}

    /// Returns the value with its bytes in reversed order.
    ///
    /// Useful for converting between big-endian and little-endian byte orders.
    public var byteSwapped: UInt64 { get {
        UInt64(raw: lang.i64_bswap(self.raw))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Compares two values for equality.
    ///
    /// Example:
    ///     (42).equals(other: 42)  // true
    ///     42 == 42                // true (operator form)
    public func equals(other: UInt64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    /// Pattern matching support. Equivalent to `equals`.
    public func matches(other: UInt64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    /// Compares this value to another, returning an Ordering.
    ///
    /// Returns `.Less` if self < other, `.Greater` if self > other,
    /// or `.Equal` if they are equal.
    public func compare(other: UInt64) -> Ordering {
        if Bool(boolLiteral: lang.i64_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i64_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Returns the next value (self + 1).
    public func successor() -> UInt64 { self.add(UInt64.one) }

    /// Returns the previous value (self - 1).
    public func predecessor() -> UInt64 { self.subtract(UInt64.one) }

    /// Creates an exclusive range from self to end (self..<end).
    public func exclusiveRange(to end: UInt64) -> Range[UInt64] {
        Range[UInt64](self, end)
    }

    /// Creates an inclusive range from self to end (self..=end).
    public func inclusiveRange(to end: UInt64) -> ClosedRange[UInt64] {
        ClosedRange[UInt64](self, end)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Hashes this value into the given hasher.
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
    type RangeConstructible.Output = Range[UInt64]
    type ClosedRangeConstructible.Output = ClosedRange[UInt64]

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    /// Adds two values. Wraps on overflow.
    public func add(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_add(self.raw, other.raw)) }

    /// Subtracts two values. Wraps on overflow.
    public func subtract(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_sub(self.raw, other.raw)) }

    /// Multiplies two values. Wraps on overflow.
    public func multiply(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_mul(self.raw, other.raw)) }

    /// Divides two values (integer division, truncates toward zero).
    public func divide(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_unsigned_div(self.raw, other.raw)) }

    /// Returns the remainder of division (self % other).
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

    /// Raises this value to the given power.
    ///
    /// Uses binary exponentiation for efficiency. Negative exponents return 0
    /// (integer division truncation).
    ///
    /// Example:
    ///     (2).pow(10)  // 1024
    ///     (3).pow(4)   // 81
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

    /// Returns the greatest common divisor of two values.
    ///
    /// Uses the Euclidean algorithm.
    ///
    /// Example:
    ///     (12).gcd(8)  // 4
    ///     (17).gcd(5)  // 1 (coprime)
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

    /// Returns the least common multiple of two values.
    ///
    /// Example:
    ///     (4).lcm(6)   // 12
    ///     (3).lcm(5)   // 15
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

    /// Clamps this value to the given range.
    ///
    /// Returns `min` if self < min, `max` if self > max, otherwise self.
    ///
    /// Example:
    ///     (5).clamp(min: 0, max: 10)   // 5
    ///     (-5).clamp(min: 0, max: 10)  // 0
    ///     (15).clamp(min: 0, max: 10)  // 10
    public func clamp(min: UInt64, max: UInt64) -> UInt64 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    /// Bitwise AND. Example: `0b1010 & 0b1100 = 0b1000`
    public func bitwiseAnd(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_and(self.raw, other.raw)) }

    /// Bitwise OR. Example: `0b1010 | 0b1100 = 0b1110`
    public func bitwiseOr(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_or(self.raw, other.raw)) }

    /// Bitwise XOR. Example: `0b1010 ^ 0b1100 = 0b0110`
    public func bitwiseXor(other: UInt64) -> UInt64 { UInt64(raw: lang.i64_xor(self.raw, other.raw)) }

    /// Bitwise NOT (complement). Flips all bits.
    public func bitwiseNot() -> UInt64 { UInt64(raw: lang.i64_not(self.raw)) }

    /// Left shift. Example: `1 << 4 = 16`
    public func shiftLeft(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_shl(self.raw, count)) }

    /// Right shift (arithmetic for signed, logical for unsigned).
    public func shiftRight(by count: lang.i64) -> UInt64 { UInt64(raw: lang.i64_unsigned_shr(self.raw, count)) }

    /// Rotates bits left by the given count.
    public func rotateLeft(by count: Int64) -> UInt64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    /// Rotates bits right by the given count.
    public func rotateRight(by count: Int64) -> UInt64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    /// `self += other`
    public mutating func addAssign(other: UInt64) { self = self.add(other) }
    /// `self -= other`
    public mutating func subtractAssign(other: UInt64) { self = self.subtract(other) }
    /// `self *= other`
    public mutating func multiplyAssign(other: UInt64) { self = self.multiply(other) }
    /// `self /= other`
    public mutating func divideAssign(other: UInt64) { self = self.divide(other) }
    /// `self %= other`
    public mutating func modAssign(other: UInt64) { self = self.modulo(other) }
    /// `self &= other`
    public mutating func bitwiseAndAssign(other: UInt64) { self = self.bitwiseAnd(other) }
    /// `self |= other`
    public mutating func bitwiseOrAssign(other: UInt64) { self = self.bitwiseOr(other) }
    /// `self ^= other`
    public mutating func bitwiseXorAssign(other: UInt64) { self = self.bitwiseXor(other) }
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
    // public static func fromBytes(bytes: Array[UInt8]) -> UInt64?
    // public static func fromBytesBigEndian(bytes: Array[UInt8]) -> UInt64?
    // public static func fromBytesLittleEndian(bytes: Array[UInt8]) -> UInt64?

    // ========================================================================
    // PARSING
    // ========================================================================

    public static func parse(string: String) -> UInt64? {
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
        let maxVal: UInt64 = UInt64.maxValue;

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

        .Some(result)
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    /// Formats this integer as a string with the given options.
    ///
    /// Supports various formatting options including radix (base), width,
    /// padding, alignment, sign display, and alternate forms.
    ///
    /// Format options:
    /// - `radix`: Number base (2, 8, 10, 16). Default: 10
    /// - `width`: Minimum output width. Default: None
    /// - `fill`: Padding character. Default: ' '
    /// - `alignment`: .Left, .Right, or .Center. Default: .Left
    /// - `sign`: .Negative (default), .Always, or .Space
    /// - `uppercase`: Use uppercase for hex digits. Default: false
    /// - `alternate`: Include prefix (0b, 0o, 0x). Default: false
    ///
    /// Example:
    ///     (42).format()  // "42"
    ///
    ///     // Hexadecimal
    ///     (255).format(options: .{radix: 16})  // "ff"
    ///     (255).format(options: .{radix: 16, uppercase: true})  // "FF"
    ///     (255).format(options: .{radix: 16, alternate: true})  // "0xff"
    ///
    ///     // Binary
    ///     (42).format(options: .{radix: 2})  // "101010"
    ///     (42).format(options: .{radix: 2, alternate: true})  // "0b101010"
    ///
    ///     // Padding and alignment
    ///     (42).format(options: .{width: .Some(5)})  // "   42"
    ///     (42).format(options: .{width: .Some(5), fill: '0'})  // "00042"
    ///     (42).format(options: .{width: .Some(5), alignment: .Left})  // "42   "
    ///
    ///     // Sign display
    ///     (42).format(options: .{sign: .Always})  // "+42"
    ///     (-42).format(options: .{sign: .Always})  // "-42"
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        var n = self;
        let isNegative = false;

        // Get radix (default 10)
        var radix: Int64 = options.radix;
        if radix < 2 or radix > 36 {
            radix = 10
        }

        // Build digits in reverse order
        var digits = String();
        if n == UInt64.zero {
            digits.appendByte(48)  // '0'
        } else {
            let radixVal: UInt64 = UInt64(from: radix);
            while n != UInt64.zero {
                let digit: UInt64 = n % radixVal;
                let digitVal: Int64 = Int64(from: digit);
                let charCode: Int64 = if digitVal < 10 {
                    digitVal + 48  // '0'-'9'
                } else if options.uppercase {
                    digitVal - 10 + 65  // 'A'-'Z'
                } else {
                    digitVal - 10 + 97  // 'a'-'z'
                };
                digits.appendByte(UInt8(from: charCode));
                n = n / radixVal
            }
        }

        // Build result string
        var result = String();

        // Add sign prefix (unsigned types only show + if requested)
        if options.sign == .Always {
            result.appendByte(43)  // '+'
        } else if options.sign == .Space {
            result.appendByte(32)  // ' '
        }

        // Add alternate form prefix (always lowercase, even with uppercase digits)
        if options.alternate {
            if radix == 2 {
                result.appendByte(48);  // '0'
                result.appendByte(98)   // 'b'
            } else if radix == 8 {
                result.appendByte(48);  // '0'
                result.appendByte(111)  // 'o'
            } else if radix == 16 {
                result.appendByte(48);  // '0'
                result.appendByte(120)  // 'x'
            }
        }

        // Append digits in correct order (reverse)
        var i = digits.byteCount - 1;
        while i >= 0 {
            result.appendByte(digits.byteAtUnchecked(i));
            i = i - 1
        }

        // Apply width and alignment padding
        if let .Some(width) = options.width {
            let currentLen = result.byteCount;
            if width > currentLen {
                let padding = width - currentLen;
                var padLeft: Int64 = 0;
                var padRight: Int64 = 0;

                if options.alignment == .Left {
                    padRight = padding
                } else if options.alignment == .Right {
                    padLeft = padding
                } else {
                    // Center
                    padLeft = padding / 2;
                    padRight = padding - padLeft
                }

                var padded = String();
                while padLeft > 0 {
                    padded.appendChar(options.fill);
                    padLeft = padLeft - 1
                }
                padded.append(result);
                while padRight > 0 {
                    padded.appendChar(options.fill);
                    padRight = padRight - 1
                }
                return padded
            }
        }

        result
    }}

/// Platform-sized unsigned integer.
///
/// On 64-bit platforms, UInt is an alias for UInt64. Use this when you need
/// an unsigned integer of the platform's native word size.
///
/// Example:
///     let size: UInt = 1024
public type UInt = UInt64
