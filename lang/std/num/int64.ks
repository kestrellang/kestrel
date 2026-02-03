// Int64 - 64-bit signed integer
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

/// A 64-bit signed integer type.
///
/// Int64 is the default integer type in Kestrel and supports arithmetic,
/// bitwise, comparison, and formatting operations. It is FFI-safe for
/// interoperability with C code.
///
/// Integer literals without a type annotation default to Int64:
///     let x = 42        // Int64
///     let y: Int8 = 42  // Int8 (explicit)
///
/// Arithmetic operations wrap on overflow (two's complement) by default.
/// Use checked methods for overflow detection or saturating methods for
/// clamping behavior.
///
/// Example:
///     let a = 100
///     let b = a + 50           // 150
///     let c = a * 2            // 200
///     let hex = 255.format(options: .{radix: 16})  // "ff"
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
    Defaultable,
    FFISafe,
    RangeConstructible,
    ClosedRangeConstructible,
    Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
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
    ///
    /// Example:
    ///     Int64.zero  // 0
    public static var zero: Int64 { Int64(intLiteral: 0) }

    /// The one value (1).
    ///
    /// Example:
    ///     Int64.one  // 1
    public static var one: Int64 { Int64(intLiteral: 1) }

    /// The minimum representable value (-9,223,372,036,854,775,808).
    ///
    /// This is -2^63. Note that `Int64.minValue.abs()` overflows because
    /// the positive value 2^63 cannot be represented in a signed 64-bit integer.
    ///
    /// Example:
    ///     Int64.minValue  // -9223372036854775808
    public static var minValue: Int64 { Int64(raw: lang.i64_shl(1, 63)) }

    /// The maximum representable value (9,223,372,036,854,775,807).
    ///
    /// This is 2^63 - 1.
    ///
    /// Example:
    ///     Int64.maxValue  // 9223372036854775807
    public static var maxValue: Int64 { Int64(intLiteral: 9223372036854775807) }

    /// The number of bits in this integer type (64).
    ///
    /// Example:
    ///     Int64.bitWidth  // 64
    public static var bitWidth: Int64 { Int64(intLiteral: 64) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates an Int64 from an integer literal.
    ///
    /// This initializer is called implicitly when using integer literals.
    ///
    /// Example:
    ///     let x: Int64 = 42
    ///     let y = Int64(intLiteral: 42)  // explicit, rarely needed
    public init(intLiteral value: lang.i64) {
        self.raw = value
    }

    /// Creates a Int64 with the default value (zero).
    public init() {
        self.init(intLiteral: 0)
    }

    /// Creates a Int64 from a raw `lang.i64` value.
    init(raw value: lang.i64) {
        self.raw = value
    }

    /// Creates an Int64 from an Int8.
    ///
    /// This conversion is always safe (widening).
    ///
    /// Example:
    ///     let small: Int8 = 42
    ///     let big = Int64(from: small)  // 42
    public init(from other: Int8) { self.raw = lang.cast_i8_i64(other.raw) }
    /// Creates an Int64 from an Int16.
    ///
    /// This conversion is always safe (widening).
    ///
    /// Example:
    ///     let small: Int16 = 1000
    ///     let big = Int64(from: small)  // 1000
    public init(from other: Int16) { self.raw = lang.cast_i16_i64(other.raw) }
    /// Creates an Int64 from an Int32.
    ///
    /// This conversion is always safe (widening).
    ///
    /// Example:
    ///     let small: Int32 = 100000
    ///     let big = Int64(from: small)  // 100000
    public init(from other: Int32) { self.raw = lang.cast_i32_i64(other.raw) }
    /// Creates an Int64 from a UInt8.
    ///
    /// This conversion is always safe (widening, unsigned to signed).
    ///
    /// Example:
    ///     let unsigned: UInt8 = 255
    ///     let signed = Int64(from: unsigned)  // 255
    public init(from other: UInt8) { self.raw = lang.cast_i8_i64(other.raw) }
    /// Creates an Int64 from a UInt16.
    ///
    /// This conversion is always safe (widening, unsigned to signed).
    ///
    /// Example:
    ///     let unsigned: UInt16 = 65535
    ///     let signed = Int64(from: unsigned)  // 65535
    public init(from other: UInt16) { self.raw = lang.cast_i16_i64(other.raw) }
    /// Creates an Int64 from a UInt32.
    ///
    /// This conversion is always safe (widening, unsigned to signed).
    ///
    /// Example:
    ///     let unsigned: UInt32 = 4000000000
    ///     let signed = Int64(from: unsigned)  // 4000000000
    public init(from other: UInt32) { self.raw = lang.cast_i32_i64(other.raw) }
    /// Creates an Int64 from a UInt64.
    ///
    /// WARNING: This conversion may overflow if the UInt64 value exceeds
    /// Int64.maxValue. The result wraps using two's complement.
    ///
    /// Example:
    ///     let small: UInt64 = 100
    ///     let signed = Int64(from: small)  // 100
    ///
    ///     let big: UInt64 = 18446744073709551615  // UInt64.maxValue
    ///     let wrapped = Int64(from: big)  // -1 (wrapped)
    public init(from other: UInt64) { self.raw = other.raw }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Returns -1 if negative, 0 if zero, or 1 if positive.
    ///
    /// Example:
    ///     (-42).sign  // -1
    ///     (0).sign    // 0
    ///     (42).sign   // 1
    public var sign: Int64 { get {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { Int64(intLiteral: lang.i64_neg(1)) }
        else if Bool(boolLiteral: lang.i64_eq(self.raw, 0)) { Int64.zero }
        else { Int64.one }
    }}

    /// Returns true if this value is greater than zero.
    ///
    /// Example:
    ///     (42).isPositive   // true
    ///     (0).isPositive    // false
    ///     (-42).isPositive  // false
    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i64_signed_gt(self.raw, 0))
    }}

    /// Returns true if this value is less than zero.
    ///
    /// Example:
    ///     (-42).isNegative  // true
    ///     (0).isNegative    // false
    ///     (42).isNegative   // false
    public var isNegative: Bool { get {
        Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0))
    }}

    /// Returns true if this value is zero.
    ///
    /// Example:
    ///     (0).isZero   // true
    ///     (42).isZero  // false
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
    ///     (2).isPowerOfTwo   // true  (2^1)
    ///     (4).isPowerOfTwo   // true  (2^2)
    ///     (3).isPowerOfTwo   // false
    ///     (0).isPowerOfTwo   // false
    ///     (-4).isPowerOfTwo  // false
    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 1)) { false }
        else { Bool(boolLiteral: lang.i64_eq(lang.i64_and(self.raw, lang.i64_sub(self.raw, 1)), 0)) }
    }}

    /// Returns the number of 1 bits in the binary representation.
    ///
    /// Also known as "population count" or "Hamming weight".
    /// For negative numbers, counts the 1 bits in two's complement.
    ///
    /// Example:
    ///     (0b1010).countOnes   // 2
    ///     (0b1111).countOnes   // 4
    ///     (0).countOnes        // 0
    ///     (-1).countOnes       // 64 (all bits set in two's complement)
    public var countOnes: Int64 { get {
        Int64(raw: lang.i64_popcount(self.raw))
    }}

    /// Returns the number of 0 bits in the binary representation.
    ///
    /// Equal to `64 - countOnes`.
    ///
    /// Example:
    ///     (0b1010).countZeros  // 62
    ///     (0).countZeros       // 64
    ///     (-1).countZeros      // 0
    public var countZeros: Int64 { get {
        Int64(intLiteral: 64) - self.countOnes
    }}

    /// Returns the number of leading zeros in the binary representation.
    ///
    /// For negative numbers (which have a leading 1 bit), returns 0.
    ///
    /// Example:
    ///     (1).leadingZeros    // 63
    ///     (256).leadingZeros  // 55 (256 = 0b100000000)
    ///     (0).leadingZeros    // 64
    ///     (-1).leadingZeros   // 0
    public var leadingZeros: Int64 { get {
        Int64(raw: lang.i64_clz(self.raw))
    }}

    /// Returns the number of trailing zeros in the binary representation.
    ///
    /// Useful for finding the largest power of 2 that divides this number.
    ///
    /// Example:
    ///     (8).trailingZeros   // 3 (8 = 0b1000)
    ///     (12).trailingZeros  // 2 (12 = 0b1100)
    ///     (1).trailingZeros   // 0
    ///     (0).trailingZeros   // 64
    public var trailingZeros: Int64 { get {
        Int64(raw: lang.i64_ctz(self.raw))
    }}

    /// Reverses the order of bytes.
    ///
    /// Useful for converting between big-endian and little-endian.
    ///
    /// Example:
    ///     (0x0102030405060708).byteSwapped  // 0x0807060504030201
    public var byteSwapped: Int64 { get {
        Int64(raw: lang.i64_bswap(self.raw))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Compares two Int64 values for equality.
    ///
    /// Example:
    ///     (42).equals(other: 42)  // true
    ///     (42).equals(other: 43)  // false
    ///     42 == 42                // true (operator form)
    public func equals(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    /// Matches this value against another in pattern matching contexts.
    ///
    /// Used by the `match` expression for integer patterns.
    ///
    /// Example:
    ///     match value {
    ///         0 => "zero",
    ///         1 => "one",
    ///         _ => "other",
    ///     }
    public func matches(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    /// Compares two Int64 values and returns their ordering.
    ///
    /// Returns `Ordering.less` if self < other, `Ordering.equal` if self == other,
    /// or `Ordering.greater` if self > other.
    ///
    /// Example:
    ///     (10).compare(other: 20)  // Ordering.less
    ///     (20).compare(other: 20)  // Ordering.equal
    ///     (30).compare(other: 20)  // Ordering.greater
    public func compare(other: Int64) -> Ordering {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i64_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Returns the next integer (self + 1).
    ///
    /// Used by ranges to iterate through values.
    ///
    /// WARNING: Wraps on overflow. `Int64.maxValue.successor()` returns
    /// `Int64.minValue`.
    ///
    /// Example:
    ///     (5).successor()   // 6
    ///     (-1).successor()  // 0
    public func successor() -> Int64 { self.add(Int64.one) }

    /// Returns the previous integer (self - 1).
    ///
    /// WARNING: Wraps on underflow. `Int64.minValue.predecessor()` returns
    /// `Int64.maxValue`.
    ///
    /// Example:
    ///     (5).predecessor()  // 4
    ///     (0).predecessor()  // -1
    public func predecessor() -> Int64 { self.subtract(Int64.one) }

    /// Creates an exclusive range from self to end (self..<end).
    public func exclusiveRange(to end: Int64) -> Range[Int64] {
        Range[Int64](self, end)
    }

    /// Creates an inclusive range from self to end (self..=end).
    public func inclusiveRange(to end: Int64) -> ClosedRange[Int64] {
        ClosedRange[Int64](self, end)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Hashes this integer into the provided hasher.
    ///
    /// Used for storing Int64 values in hash-based collections like
    /// Dictionary and Set.
    ///
    /// Example:
    ///     let set = Set[Int64]()
    ///     set.insert(element: 42)  // uses hash() internally
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
    type RangeConstructible.Output = Range[Int64]
    type ClosedRangeConstructible.Output = ClosedRange[Int64]

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    /// Adds two integers.
    ///
    /// WARNING: Wraps on overflow using two's complement arithmetic.
    ///
    /// Example:
    ///     (10).add(other: 5)   // 15
    ///     10 + 5               // 15 (operator form)
    ///
    ///     // Overflow wraps:
    ///     Int64.maxValue.add(other: 1)  // Int64.minValue
    public func add(other: Int64) -> Int64 { Int64(raw: lang.i64_add(self.raw, other.raw)) }

    /// Subtracts another integer from this one.
    ///
    /// WARNING: Wraps on underflow using two's complement arithmetic.
    ///
    /// Example:
    ///     (10).subtract(other: 3)  // 7
    ///     10 - 3                   // 7 (operator form)
    ///
    ///     // Underflow wraps:
    ///     Int64.minValue.subtract(other: 1)  // Int64.maxValue
    public func subtract(other: Int64) -> Int64 { Int64(raw: lang.i64_sub(self.raw, other.raw)) }

    /// Multiplies two integers.
    ///
    /// WARNING: Wraps on overflow using two's complement arithmetic.
    ///
    /// Example:
    ///     (10).multiply(other: 5)  // 50
    ///     10 * 5                   // 50 (operator form)
    public func multiply(other: Int64) -> Int64 { Int64(raw: lang.i64_mul(self.raw, other.raw)) }

    /// Divides this integer by another (truncating toward zero).
    ///
    /// WARNING: Panics if other is zero.
    ///
    /// Note: Division truncates toward zero, not toward negative infinity.
    /// This means `-7 / 2 = -3`, not `-4`.
    ///
    /// Example:
    ///     (10).divide(other: 3)   // 3
    ///     10 / 3                  // 3 (operator form)
    ///     (-7).divide(other: 2)   // -3 (truncates toward zero)
    ///     (7).divide(other: -2)   // -3
    public func divide(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_div(self.raw, other.raw)) }

    /// Returns the remainder after division (truncating toward zero).
    ///
    /// WARNING: Panics if other is zero.
    ///
    /// The sign of the result matches the sign of the dividend (self).
    ///
    /// Example:
    ///     (10).modulo(other: 3)   // 1
    ///     10 % 3                  // 1 (operator form)
    ///     (-10).modulo(other: 3)  // -1
    ///     (10).modulo(other: -3)  // 1
    public func modulo(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_rem(self.raw, other.raw)) }

    /// Returns the negation of this integer.
    ///
    /// WARNING: `Int64.minValue.negate()` overflows and returns `Int64.minValue`
    /// because the positive value cannot be represented.
    ///
    /// Example:
    ///     (42).negate()   // -42
    ///     (-42).negate()  // 42
    ///     -42             // -42 (operator form)
    public func negate() -> Int64 { Int64(raw: lang.i64_neg(self.raw)) }
    /// Returns the absolute value of this integer.
    ///
    /// WARNING: `Int64.minValue.abs()` overflows and returns `Int64.minValue`
    /// because the positive value 2^63 cannot be represented in Int64.
    ///
    /// Example:
    ///     (42).abs()   // 42
    ///     (-42).abs()  // 42
    ///     (0).abs()    // 0
    public func abs() -> Int64 { if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    /// Adds two integers, returning None on overflow.
    ///
    /// Use this when overflow is a possible error condition that should
    /// be handled explicitly.
    ///
    /// Example:
    ///     (10).addChecked(other: 5)  // Some(15)
    ///     Int64.maxValue.addChecked(other: 1)  // None
    ///
    ///     if let result = a.addChecked(other: b) {
    ///         // use result safely
    ///     } else {
    ///         // handle overflow
    ///     }
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

    /// Subtracts another integer, returning None on underflow.
    ///
    /// Example:
    ///     (10).subtractChecked(other: 3)  // Some(7)
    ///     Int64.minValue.subtractChecked(other: 1)  // None
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

    /// Multiplies two integers, returning None on overflow.
    ///
    /// Example:
    ///     (10).multiplyChecked(other: 5)  // Some(50)
    ///     Int64.maxValue.multiplyChecked(other: 2)  // None
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

    /// Divides two integers, returning None on division by zero.
    ///
    /// Also returns None for the special case of `Int64.minValue / -1`
    /// which would overflow.
    ///
    /// Example:
    ///     (10).divideChecked(other: 3)  // Some(3)
    ///     (10).divideChecked(other: 0)  // None
    ///     Int64.minValue.divideChecked(other: -1)  // None
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

    /// Returns the negation, or None if it would overflow.
    ///
    /// Only `Int64.minValue.negateChecked()` returns None.
    ///
    /// Example:
    ///     (42).negateChecked()   // Some(-42)
    ///     (-42).negateChecked()  // Some(42)
    ///     Int64.minValue.negateChecked()  // None
    public func negateChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.negate())
    }

    /// Returns the absolute value, or None if it would overflow.
    ///
    /// Only `Int64.minValue.absChecked()` returns None.
    ///
    /// Example:
    ///     (42).absChecked()   // Some(42)
    ///     (-42).absChecked()  // Some(42)
    ///     Int64.minValue.absChecked()  // None
    public func absChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.abs())
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    /// Adds two integers, clamping the result to [minValue, maxValue].
    ///
    /// Never wraps - if the true result would overflow, returns maxValue.
    /// If it would underflow, returns minValue.
    ///
    /// Example:
    ///     (10).addSaturating(other: 5)  // 15
    ///     Int64.maxValue.addSaturating(other: 1)  // Int64.maxValue
    ///     Int64.maxValue.addSaturating(other: 100)  // Int64.maxValue
    public func addSaturating(other: Int64) -> Int64 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isPositive { Int64.maxValue } else { Int64.minValue }
        }
    }

    /// Subtracts another integer, clamping the result to [minValue, maxValue].
    ///
    /// Example:
    ///     (10).subtractSaturating(other: 3)  // 7
    ///     Int64.minValue.subtractSaturating(other: 1)  // Int64.minValue
    public func subtractSaturating(other: Int64) -> Int64 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isNegative { Int64.maxValue } else { Int64.minValue }
        }
    }

    /// Multiplies two integers, clamping the result to [minValue, maxValue].
    ///
    /// Example:
    ///     (10).multiplySaturating(other: 5)  // 50
    ///     Int64.maxValue.multiplySaturating(other: 2)  // Int64.maxValue
    ///     Int64.maxValue.multiplySaturating(other: -2)  // Int64.minValue
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

    /// Returns the negation, clamping to maxValue if it would overflow.
    ///
    /// Example:
    ///     (42).negateSaturating()   // -42
    ///     Int64.minValue.negateSaturating()  // Int64.maxValue
    public func negateSaturating() -> Int64 {
        if self == Int64.minValue {
            Int64.maxValue
        } else {
            self.negate()
        }
    }

    /// Returns the absolute value, clamping to maxValue if it would overflow.
    ///
    /// Example:
    ///     (-42).absSaturating()  // 42
    ///     Int64.minValue.absSaturating()  // Int64.maxValue
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

    /// Returns this integer raised to the given power.
    ///
    /// WARNING: Wraps on overflow. For large exponents, consider using
    /// checked arithmetic or a big integer library.
    ///
    /// Example:
    ///     (2).pow(exponent: 10)  // 1024
    ///     (3).pow(exponent: 4)   // 81
    ///     (5).pow(exponent: 0)   // 1
    ///     (-2).pow(exponent: 3)  // -8
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

    /// Returns the greatest common divisor of two integers.
    ///
    /// The result is always non-negative. Returns the other value if
    /// either input is zero.
    ///
    /// Example:
    ///     (12).gcd(other: 8)   // 4
    ///     (17).gcd(other: 13)  // 1 (coprime)
    ///     (0).gcd(other: 5)    // 5
    ///     (-12).gcd(other: 8)  // 4
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

    /// Returns the least common multiple of two integers.
    ///
    /// The result is always non-negative. Returns 0 if either input is zero.
    ///
    /// WARNING: May overflow for large inputs.
    ///
    /// Example:
    ///     (4).lcm(other: 6)   // 12
    ///     (3).lcm(other: 5)   // 15
    ///     (0).lcm(other: 5)   // 0
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

    /// Returns this value clamped to the given range.
    ///
    /// If self < min, returns min. If self > max, returns max.
    /// Otherwise returns self unchanged.
    ///
    /// Panics if min > max.
    ///
    /// Example:
    ///     (5).clamp(min: 0, max: 10)   // 5
    ///     (-5).clamp(min: 0, max: 10)  // 0
    ///     (15).clamp(min: 0, max: 10)  // 10
    public func clamp(min: Int64, max: Int64) -> Int64 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    /// Returns the bitwise AND of two integers.
    ///
    /// Each bit in the result is 1 only if both corresponding bits are 1.
    ///
    /// Example:
    ///     (0b1100).bitwiseAnd(other: 0b1010)  // 0b1000 (8)
    ///     0b1100 & 0b1010                     // 0b1000 (operator form)
    ///
    ///     // Common use: masking bits
    ///     let masked = value & 0xFF  // keep lowest 8 bits
    public func bitwiseAnd(other: Int64) -> Int64 { Int64(raw: lang.i64_and(self.raw, other.raw)) }

    /// Returns the bitwise OR of two integers.
    ///
    /// Each bit in the result is 1 if either corresponding bit is 1.
    ///
    /// Example:
    ///     (0b1100).bitwiseOr(other: 0b1010)  // 0b1110 (14)
    ///     0b1100 | 0b1010                    // 0b1110 (operator form)
    ///
    ///     // Common use: setting bits
    ///     let flags = flags | FLAG_ENABLED
    public func bitwiseOr(other: Int64) -> Int64 { Int64(raw: lang.i64_or(self.raw, other.raw)) }

    /// Returns the bitwise XOR of two integers.
    ///
    /// Each bit in the result is 1 if the corresponding bits differ.
    ///
    /// Example:
    ///     (0b1100).bitwiseXor(other: 0b1010)  // 0b0110 (6)
    ///     0b1100 ^ 0b1010                     // 0b0110 (operator form)
    ///
    ///     // Common use: toggling bits
    ///     let toggled = flags ^ FLAG_ENABLED
    public func bitwiseXor(other: Int64) -> Int64 { Int64(raw: lang.i64_xor(self.raw, other.raw)) }

    /// Returns the bitwise NOT (complement) of this integer.
    ///
    /// Inverts all bits. Equivalent to `-(self + 1)` for signed integers.
    ///
    /// Example:
    ///     (0).bitwiseNot()   // -1 (all bits set)
    ///     (-1).bitwiseNot()  // 0
    ///     ~0                 // -1 (operator form)
    public func bitwiseNot() -> Int64 { Int64(raw: lang.i64_not(self.raw)) }

    /// Shifts bits left by the given count, filling with zeros.
    ///
    /// Equivalent to multiplication by 2^count (ignoring overflow).
    ///
    /// Behavior for count < 0 or count >= 64 is undefined.
    ///
    /// Example:
    ///     (1).shiftLeft(by: 4)   // 16 (0b10000)
    ///     1 << 4                 // 16 (operator form)
    ///     (0b0011).shiftLeft(by: 2)  // 0b1100 (12)
    public func shiftLeft(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_shl(self.raw, count)) }

    /// Shifts bits right by the given count (arithmetic shift).
    ///
    /// For positive numbers, fills with zeros. For negative numbers,
    /// fills with ones (sign extension), preserving the sign.
    ///
    /// Behavior for count < 0 or count >= 64 is undefined.
    ///
    /// Example:
    ///     (16).shiftRight(by: 2)   // 4
    ///     16 >> 2                  // 4 (operator form)
    ///     (-16).shiftRight(by: 2)  // -4 (sign preserved)
    public func shiftRight(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_signed_shr(self.raw, count)) }

    /// Rotates bits left by the given count.
    ///
    /// Bits shifted out on the left reappear on the right.
    ///
    /// Example:
    ///     (0x1234567890ABCDEF).rotateLeft(by: 8)  // 0x34567890ABCDEF12
    public func rotateLeft(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    /// Rotates bits right by the given count.
    ///
    /// Bits shifted out on the right reappear on the left.
    ///
    /// Example:
    ///     (0x1234567890ABCDEF).rotateRight(by: 8)  // 0xEF1234567890ABCD
    public func rotateRight(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    /// Adds another integer to this one in place.
    ///
    /// Equivalent to `self = self + other`.
    ///
    /// Example:
    ///     var x = 10
    ///     x.addAssign(other: 5)  // x is now 15
    ///     x += 5                 // x is now 20 (operator form)
    public mutating func addAssign(other: Int64) { self = self.add(other) }
    /// Subtracts another integer from this one in place.
    ///
    /// Equivalent to `self = self - other`.
    ///
    /// Example:
    ///     var x = 10
    ///     x.subtractAssign(other: 3)  // x is now 7
    ///     x -= 3                      // x is now 4 (operator form)
    public mutating func subtractAssign(other: Int64) { self = self.subtract(other) }
    /// Multiplies this integer by another in place.
    ///
    /// Equivalent to `self = self * other`.
    ///
    /// Example:
    ///     var x = 10
    ///     x.multiplyAssign(other: 5)  // x is now 50
    ///     x *= 2                      // x is now 100 (operator form)
    public mutating func multiplyAssign(other: Int64) { self = self.multiply(other) }
    /// Divides this integer by another in place.
    ///
    /// Equivalent to `self = self / other`.
    ///
    /// WARNING: Panics if other is zero.
    ///
    /// Example:
    ///     var x = 20
    ///     x.divideAssign(other: 4)  // x is now 5
    ///     x /= 5                    // x is now 1 (operator form)
    public mutating func divideAssign(other: Int64) { self = self.divide(other) }
    /// Computes the remainder of division in place.
    ///
    /// Equivalent to `self = self % other`.
    ///
    /// WARNING: Panics if other is zero.
    ///
    /// Example:
    ///     var x = 17
    ///     x.modAssign(other: 5)  // x is now 2
    ///     x %= 2                 // x is now 0 (operator form)
    public mutating func modAssign(other: Int64) { self = self.modulo(other) }
    /// Performs bitwise AND in place.
    ///
    /// Equivalent to `self = self & other`.
    ///
    /// Example:
    ///     var x = 0b1111
    ///     x.bitwiseAndAssign(other: 0b1010)  // x is now 0b1010
    ///     x &= 0b1100                        // x is now 0b1000 (operator form)
    public mutating func bitwiseAndAssign(other: Int64) { self = self.bitwiseAnd(other) }
    /// Performs bitwise OR in place.
    ///
    /// Equivalent to `self = self | other`.
    ///
    /// Example:
    ///     var x = 0b1100
    ///     x.bitwiseOrAssign(other: 0b0011)  // x is now 0b1111
    ///     x |= 0b0001                       // still 0b1111 (operator form)
    public mutating func bitwiseOrAssign(other: Int64) { self = self.bitwiseOr(other) }
    /// Performs bitwise XOR in place.
    ///
    /// Equivalent to `self = self ^ other`.
    ///
    /// Example:
    ///     var x = 0b1111
    ///     x.bitwiseXorAssign(other: 0b1010)  // x is now 0b0101
    ///     x ^= 0b0101                        // x is now 0b0000 (operator form)
    public mutating func bitwiseXorAssign(other: Int64) { self = self.bitwiseXor(other) }
    /// Shifts bits left in place.
    ///
    /// Equivalent to `self = self << count`.
    ///
    /// Example:
    ///     var x = 1
    ///     x.shiftLeftAssign(by: 4)  // x is now 16
    ///     x <<= 1                   // x is now 32 (operator form)
    public mutating func shiftLeftAssign(by count: lang.i64) { self = self.shiftLeft(by: count) }
    /// Shifts bits right in place.
    ///
    /// Equivalent to `self = self >> count`.
    ///
    /// Example:
    ///     var x = 32
    ///     x.shiftRightAssign(by: 2)  // x is now 8
    ///     x >>= 1                    // x is now 4 (operator form)
    public mutating func shiftRightAssign(by count: lang.i64) { self = self.shiftRight(by: count) }

    // ========================================================================
    // BYTE CONVERSION
    // ========================================================================

    /// Returns this integer as an array of 8 bytes in native byte order.
    ///
    /// Example:
    ///     let bytes = (0x0102030405060708).toBytes()
    ///     // On little-endian: [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]
    ///     // On big-endian: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    public func toBytes() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 8);
        let value = self;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 8 {
            result.append(ptr.offset(by: i).read());
            i = i + 1
        }
        result
    }

    /// Returns this integer as an array of 8 bytes in big-endian order.
    ///
    /// Big-endian: most significant byte first (network byte order).
    ///
    /// Example:
    ///     (0x0102030405060708).toBytesBigEndian()
    ///     // [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    public func toBytesBigEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 8);
        let value = UInt64(raw: self.raw);
        let mask = UInt64(intLiteral: 255);
        var i: Int64 = 0;
        while i < 8 {
            let shift = (Int64(intLiteral: 7) - i) * Int64(intLiteral: 8);
            let byteVal = value.shiftRight(by: shift.raw).bitwiseAnd(mask);
            result.append(UInt8(from: byteVal));
            i = i + 1
        }
        result
    }

    /// Returns this integer as an array of 8 bytes in little-endian order.
    ///
    /// Little-endian: least significant byte first.
    ///
    /// Example:
    ///     (0x0102030405060708).toBytesLittleEndian()
    ///     // [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]
    public func toBytesLittleEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 8);
        let value = UInt64(raw: self.raw);
        let mask = UInt64(intLiteral: 255);
        var i: Int64 = 0;
        while i < 8 {
            let shift = i * Int64(intLiteral: 8);
            let byteVal = value.shiftRight(by: shift.raw).bitwiseAnd(mask);
            result.append(UInt8(from: byteVal));
            i = i + 1
        }
        result
    }

    /// Creates an Int64 from an array of 8 bytes in native byte order.
    ///
    /// Returns None if the array doesn't have exactly 8 bytes.
    ///
    /// Example:
    ///     Int64.fromBytes(bytes: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
    public static func fromBytes(bytes: std.collections.Array[UInt8]) -> Int64? {
        if bytes.count != Int64(intLiteral: 8) {
            return .None
        }

        var value = Int64(intLiteral: 0);
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 8 {
            ptr.offset(by: i).write(bytes(unchecked: i));
            i = i + 1
        }
        .Some(value)
    }

    /// Creates an Int64 from an array of 8 bytes in big-endian order.
    ///
    /// Returns None if the array doesn't have exactly 8 bytes.
    ///
    /// Example:
    ///     Int64.fromBytesBigEndian(bytes: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
    ///     // 0x0102030405060708
    public static func fromBytesBigEndian(bytes: std.collections.Array[UInt8]) -> Int64? {
        if bytes.count != Int64(intLiteral: 8) {
            return .None
        }

        var result = UInt64(intLiteral: 0);
        var i: Int64 = 0;
        while i < 8 {
            let byteVal = UInt64(from: bytes(unchecked: i));
            result = result.shiftLeft(by: Int64(intLiteral: 8).raw).bitwiseOr(byteVal);
            i = i + 1
        }
        .Some(Int64(from: result))
    }

    /// Creates an Int64 from an array of 8 bytes in little-endian order.
    ///
    /// Returns None if the array doesn't have exactly 8 bytes.
    ///
    /// Example:
    ///     Int64.fromBytesLittleEndian(bytes: [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01])
    ///     // 0x0102030405060708
    public static func fromBytesLittleEndian(bytes: std.collections.Array[UInt8]) -> Int64? {
        if bytes.count != Int64(intLiteral: 8) {
            return .None
        }

        var result = UInt64(intLiteral: 0);
        var i: Int64 = 0;
        while i < 8 {
            let shift = i * Int64(intLiteral: 8);
            let byteVal = UInt64(from: bytes(unchecked: i));
            result = result.bitwiseOr(byteVal.shiftLeft(by: shift.raw));
            i = i + 1
        }
        .Some(Int64(from: result))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    /// Parses an integer from a string in base 10.
    ///
    /// Returns None if the string is not a valid integer or if the value
    /// would overflow Int64.
    ///
    /// Accepts optional leading '+' or '-' sign, followed by one or more digits.
    /// Leading/trailing whitespace is not allowed.
    ///
    /// Example:
    ///     Int64.parse(string: "42")      // Some(42)
    ///     Int64.parse(string: "-42")     // Some(-42)
    ///     Int64.parse(string: "+42")     // Some(42)
    ///     Int64.parse(string: "abc")     // None
    ///     Int64.parse(string: "")        // None
    ///     Int64.parse(string: " 42")     // None (whitespace)
    ///     Int64.parse(string: "99999999999999999999")  // None (overflow)
    public static func parse(string: String) -> Int64? {
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
            if result < Int64.minValue {
                return .None
            }
        } else {
            if result > Int64.maxValue {
                return .None
            }
        }

        .Some(result)
    }
    /// Parses an integer from a string in the given radix (base).
    ///
    /// Radix must be between 2 and 36 inclusive. For radix > 10, letters
    /// a-z (case insensitive) represent values 10-35.
    ///
    /// Example:
    ///     Int64.parse(string: "ff", radix: 16)    // Some(255)
    ///     Int64.parse(string: "FF", radix: 16)    // Some(255)
    ///     Int64.parse(string: "101010", radix: 2) // Some(42)
    ///     Int64.parse(string: "z", radix: 36)     // Some(35)
    public static func parse(string: String, radix: Int64) -> Int64? {
        if radix < 2 or radix > 36 {
            return .None
        }

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

        let radixU: UInt64 = UInt64(from: radix);
        let maxMagnitude: UInt64 = if isNegative {
            UInt64(from: Int64.maxValue) + UInt64(intLiteral: 1)
        } else {
            UInt64(from: Int64.maxValue)
        };

        var result: UInt64 = 0;

        while index < len {
            let byte: UInt8 = string.byteAtUnchecked(index);
            let byteVal = Int64(from: byte);

            let digit: Int64 = if byteVal >= 48 and byteVal <= 57 {
                byteVal - 48
            } else if byteVal >= 65 and byteVal <= 90 {
                byteVal - 55
            } else if byteVal >= 97 and byteVal <= 122 {
                byteVal - 87
            } else {
                return .None
            };

            if digit >= radix {
                return .None
            }

            let digitU: UInt64 = UInt64(from: digit);
            if result > (maxMagnitude - digitU) / radixU {
                return .None
            }
            result = result * radixU + digitU;
            index = index + 1
        }

        let signedResult = Int64(from: result);
        if isNegative {
            .Some(signedResult.negate())
        } else {
            .Some(signedResult)
        }
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    /// Formats this integer as a string.
    ///
    /// Supports various formatting options including radix (base), width,
    /// padding, alignment, sign display, and alternate forms.
    ///
    /// Format options:
    /// - `radix`: Number base (2, 8, 10, 16). Default: 10
    /// - `width`: Minimum output width. Default: 0
    /// - `fill`: Padding character. Default: ' '
    /// - `alignment`: .left, .right, or .center. Default: .right
    /// - `sign`: .negative (default), .always, or .space
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
    ///     (42).format(options: .{width: 5})  // "   42"
    ///     (42).format(options: .{width: 5, fill: '0'})  // "00042"
    ///     (42).format(options: .{width: 5, alignment: .left})  // "42   "
    ///
    ///     // Sign display
    ///     (42).format(options: .{sign: .always})  // "+42"
    ///     (-42).format(options: .{sign: .always})  // "-42"
    ///
    ///     // String interpolation
    ///     "\{value}"           // decimal
    ///     "\{value:x}"         // hexadecimal
    ///     "\{value:#x}"        // hexadecimal with 0x prefix
    ///     "\{value:08}"        // zero-padded to 8 digits
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
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
        var i = result.byteCount - 1;
        while i >= 0 {
            reversed.appendByte(result.byteAtUnchecked(i));
            i = i - 1
        }
        reversed
    }}

/// Platform-sized signed integer.
///
/// On 64-bit platforms, Int is an alias for Int64. This is the recommended
/// integer type for most use cases.
///
/// Example:
///     let count: Int = 100
///     for i in 0..count { ... }
public type Int = Int64
