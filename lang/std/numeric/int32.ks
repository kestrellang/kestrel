// Int32 - 32-bit signed integer
// Generated from integer.ks.template (docs synced from .ks.interface) - DO NOT EDIT

module std.numeric

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable, Hashable, Hasher,
    Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    AddAssign, SubtractAssign, MultiplyAssign, DivideAssign, ModuloAssign,
    BitwiseAndAssign, BitwiseOrAssign, BitwiseXorAssign, LeftShiftAssign, RightShiftAssign,
    ExpressibleByIntLiteral, Convertible, Defaultable,
    RangeConstructible, ClosedRangeConstructible, Range, ClosedRange,
    RangeFromConstructible, RangeUpToConstructible, RangeThroughConstructible,
    RangeFrom, RangeUpTo, RangeThrough
)
import std.text.(String, StringBuilder, Formattable, FormatOptions, _writePadded)
import std.memory.(ArraySlice, Pointer)
import std.collections.(Slice)
import std.numeric.(UInt8, Int64, UInt64)

/// A 32-bit signed integer.
///
/// Int32 is the 32-bit member of the integer family. The same surface
/// area is provided across all widths; switch widths to trade range for memory
/// or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
/// `*Checked` variants for overflow detection or `*Saturating` to clamp to
/// `minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
/// `lang.i32` so it can cross C boundaries unchanged.
///
/// # Examples
///
/// ```
/// let a: Int64 = 100;
/// let b = a + 50;        // 150
/// let c = a * 2;         // 200
/// let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
/// ```
///
/// ```
/// // Bit twiddling
/// (0b1010).countOnes      // 2
/// (1).shiftLeft(by: 4)    // 16
/// (-1).leadingZeros       // 0  (all bits set)
/// ```
///
/// # Representation
///
/// A single `lang.i32` field. No padding, no headers — bit-identical
/// to the corresponding C type.
public struct Int32:
    SignedInteger,
    Steppable,
    Comparable,
    Equatable,
    Matchable,
    Formattable,
    Hashable,
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
    LeftShiftAssign[Int64],
    RightShiftAssign[Int64],
    ExpressibleByIntLiteral,
    Defaultable,
    FFISafe,
    RangeConstructible,
    ClosedRangeConstructible,
    RangeFromConstructible,
    RangeUpToConstructible,
    RangeThroughConstructible,
    Convertible[Int8],
    Convertible[Int16],
    Convertible[Int64],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    Convertible[UInt64]
{
    /// The underlying primitive `lang.i32` value. Exposed for FFI
    /// and intrinsic use; prefer the typed surface for everything else.
    public var raw: lang.i32

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    /// The additive identity, `0`.
    public static var zero: Int32 { 0 }

    /// The multiplicative identity, `1`.
    public static var one: Int32 { 1 }

    /// The smallest representable value.
    /// This is -2^31 (-2_147_483_648).
    /// Note that for signed types `minValue.negate()` overflows back to
    /// itself; use `negateChecked()` if you need to detect that.
    public static var minValue: Int32 { Int32(raw: lang.i32_shl(1, 31)) }

    /// The largest representable value.
    /// This is 2^31 - 1 (2_147_483_647).
    public static var maxValue: Int32 { 2147483647 }

    /// The width in bits (32). Useful for shift bounds and bit-walks.
    public static var bitWidth: Int64 { 32 }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// @name Int Literal
    /// Compiler-emitted bridge that turns an integer literal into a Int32.
    ///
    /// You will rarely call this directly — write the literal and let the
    /// `ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
    /// 64 bits the literal is truncated with `lang.cast_i64_i32`.
    ///
    /// # Examples
    ///
    /// ```
    /// let n: Int64 = 42;            // implicit
    /// ```
    public init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i32(value)
    }

    /// @name Default
    /// Creates the zero value, satisfying `Defaultable`.
    ///
    /// # Examples
    ///
    /// ```
    /// let n = Int64();   // 0
    /// ```
    public init() {
        self.init(intLiteral: 0)
    }

    /// @name From Raw
    /// Wraps an existing `lang.i32` without conversion. Internal
    /// constructor used by intrinsics; not part of the public API.
    init(raw value: lang.i32) {
        self.raw = value
    }

    /// @name From Integer
    /// Converts from `Int8`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: Int8) { self.raw = lang.cast_i8_i32(other.raw) }
    /// @name From Integer
    /// Converts from `Int16`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: Int16) { self.raw = lang.cast_i16_i32(other.raw) }
    /// @name From Integer
    /// Converts from `Int64`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: Int64) { self.raw = lang.cast_i64_i32(other.raw) }
    /// @name From Integer
    /// Converts from `UInt8`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt8) { self.raw = lang.cast_u8_i32(other.raw) }
    /// @name From Integer
    /// Converts from `UInt16`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt16) { self.raw = lang.cast_u16_i32(other.raw) }
    /// @name From Integer
    /// Converts from `UInt32`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt32) { self.raw = other.raw }
    /// @name From Integer
    /// Converts from `UInt64`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt64) { self.raw = lang.cast_u64_i32(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Sign as a `Int32`: `-1`, `0`, or `1`.
    public var sign: Int32 { get {
        if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0)) { Int32(intLiteral: lang.i64_neg(1)) }
        else if Bool(boolLiteral: lang.i32_eq(self.raw, 0)) { Int32.zero }
        else { Int32.one }
    }}

    /// True when `self > 0`.
    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i32_signed_gt(self.raw, 0))
    }}

    /// True when `self < 0`.
    public var isNegative: Bool { get {
        Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0))
    }}

    /// True when `self == 0`.
    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i32_eq(self.raw, 0))
    }}

    // ========================================================================
    // BIT INSPECTION (Properties)
    // ========================================================================

    /// True when the value is a positive power of two (`2^k` for `k >= 0`).
    ///
    /// Zero and negatives are excluded. Cheap branchless test built on
    /// `x & (x - 1) == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// (1).isPowerOfTwo;   // true  (2^0)
    /// (4).isPowerOfTwo;   // true  (2^2)
    /// (3).isPowerOfTwo;   // false
    /// (0).isPowerOfTwo;   // false
    /// ```
    public var isPowerOfTwo: Bool { get {
        if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 1)) { false }
        else { Bool(boolLiteral: lang.i32_eq(lang.i32_and(self.raw, lang.i32_sub(self.raw, 1)), 0)) }
    }}

    /// Population count — the number of `1` bits in the binary representation.
    ///
    /// Lowered to a `popcount` intrinsic where the target supports it.
    ///
    /// # Examples
    ///
    /// ```
    /// (0b1010).countOnes;  // 2
    /// (0b1111).countOnes;  // 4
    /// (0).countOnes;       // 0
    /// ```
    public var countOnes: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_popcount(self.raw)))
    }}

    /// Complement of `countOnes`: equal to `bitWidth - countOnes`.
    public var countZeros: Int64 { get {
        32 - self.countOnes
    }}

    /// Number of leading zero bits, counting from the most-significant end.
    ///
    /// For zero, returns `bitWidth`.
    ///
    /// # Examples
    ///
    /// ```
    /// (1).leadingZeros;   // bitWidth - 1
    /// (0).leadingZeros;   // bitWidth
    /// ```
    public var leadingZeros: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_clz(self.raw)))
    }}

    /// Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
    /// values; returns `bitWidth` for zero. Useful for finding the largest
    /// power of two dividing the value.
    public var trailingZeros: Int64 { get {
        Int64(raw: lang.cast_i32_i64(lang.i32_ctz(self.raw)))
    }}

    /// Value with its byte order reversed. Use to convert between big- and
    /// little-endian; lowered to a `bswap` intrinsic.
    public var byteSwapped: Int32 { get {
        Int32(raw: lang.i32_bswap(self.raw))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Bit-for-bit equality. Backs the `==` operator.
    ///
    /// # Examples
    ///
    /// ```
    /// (42).isEqual(to: 42);  // true
    /// 42 == 42;               // true
    /// ```
    public func isEqual(to other: Int32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    /// Pattern-matching hook for `Matchable`. Identical to `isEqual`.
    public func matches(other: Int32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    /// Three-way comparison returning an `Ordering`. Signed types compare
    /// using two's-complement ordering; unsigned types use natural ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// (1).compare(2);   // .Less
    /// (2).compare(2);   // .Equal
    /// (3).compare(2);   // .Greater
    /// ```
    public func compare(other: Int32) -> Ordering {
        if Bool(boolLiteral: lang.i32_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i32_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
    /// integer ranges.
    public func successor() -> Int32 { self.add(Int32.one) }

    /// Predecessor — `self - 1`. Wraps at `minValue`.
    public func predecessor() -> Int32 { self.subtract(Int32.one) }

    /// Builds a half-open range `self..<end`. Sugar for the `..<` operator.
    public func exclusiveRange(to end: Int32) -> Range[Int32] {
        Range[Int32](self, end)
    }

    /// Builds a closed range `self..=end`. Sugar for the `..=` operator.
    public func inclusiveRange(to end: Int32) -> ClosedRange[Int32] {
        ClosedRange[Int32](self, end)
    }

    /// Builds a partial range `self..` (from self, no upper bound).
    public func rangeFrom() -> RangeFrom[Int32] {
        RangeFrom[Int32](self)
    }

    /// Builds a partial range `..<self` (up to self, exclusive).
    public func rangeUpTo() -> RangeUpTo[Int32] {
        RangeUpTo[Int32](self)
    }

    /// Builds a partial range `..=self` (through self, inclusive).
    public func rangeThrough() -> RangeThrough[Int32] {
        RangeThrough[Int32](self)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
    /// only within a single process — do not persist hashes across builds.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(ArraySlice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Layout.of[Int32]().size))
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
    type RangeConstructible.Output = Range[Int32]
    type ClosedRangeConstructible.Output = ClosedRange[Int32]
    type RangeFromConstructible.Output = RangeFrom[Int32]
    type RangeUpToConstructible.Output = RangeUpTo[Int32]
    type RangeThroughConstructible.Output = RangeThrough[Int32]

    // ========================================================================
    // ARITHMETIC (Wrapping - Default)
    // ========================================================================

    /// `self + other`, wrapping on overflow. Use `addChecked` to detect or
    /// `addSaturating` to clamp.
    public consuming func add(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_add(self.raw, other.raw)) }

    /// `self - other`, wrapping on overflow.
    public consuming func subtract(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_sub(self.raw, other.raw)) }

    /// `self * other`, wrapping on overflow.
    public consuming func multiply(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_mul(self.raw, other.raw)) }

    /// Truncating integer division (`self / other`). For signed types,
    /// `minValue / -1` wraps; use `divideChecked` to detect.
    ///
    /// # Errors
    ///
    /// Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
    /// process aborts before producing a result).
    public consuming func divide(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_signed_div(self.raw, other.raw)) }

    /// `self % other` — truncated remainder; the result has the sign of
    /// `self` for signed types.
    ///
    /// # Errors
    ///
    /// Traps on division by zero, like `divide`.
    public consuming func modulo(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_signed_rem(self.raw, other.raw)) }

    /// Two's-complement negation. Wraps at the minimum value:
    /// `Int32.minValue.negate() == Int32.minValue`. Use
    /// `negateChecked` to surface the overflow.
    public consuming func negate() -> Int32 { Int32(raw: lang.i32_neg(self.raw)) }
    /// Absolute value. Wraps at the minimum value
    /// (`Int32.minValue.abs() == Int32.minValue`); use
    /// `absChecked` if that's a problem.
    public func abs() -> Int32 { if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    /// Wrapping addition that returns `None` instead of overflowing.
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

    /// Wrapping subtraction that returns `None` instead of overflowing.
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

    /// Wrapping multiplication that returns `None` instead of overflowing.
    /// Implemented by multiplying then dividing back; replace with an
    /// overflow-detecting intrinsic when one is available.
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

    /// Division that returns `None` for divide-by-zero or for the
    /// `minValue / -1` overflow case.
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

    /// Negation that returns `None` for `minValue` (whose negation overflows).
    public func negateChecked() -> Int32? {
        if self == Int32.minValue {
            return .None
        };
        .Some(self.negate())
    }

    /// Absolute value that returns `None` for `minValue` (whose absolute
    /// value overflows).
    public func absChecked() -> Int32? {
        if self == Int32.minValue {
            return .None
        };
        .Some(self.abs())
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    /// Addition that clamps to `maxValue`/`minValue` instead of wrapping.
    public func addSaturating(other: Int32) -> Int32 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isPositive { Int32.maxValue } else { Int32.minValue }
        }
    }

    /// Subtraction that clamps to `maxValue`/`minValue` instead of wrapping.
    public func subtractSaturating(other: Int32) -> Int32 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isNegative { Int32.maxValue } else { Int32.minValue }
        }
    }

    /// Multiplication that clamps to `maxValue`/`minValue` instead of wrapping.
    /// The clamp direction follows the algebraic sign of the would-be result.
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

    /// Negation that returns `maxValue` instead of wrapping `minValue`.
    public func negateSaturating() -> Int32 {
        if self == Int32.minValue {
            Int32.maxValue
        } else {
            self.negate()
        }
    }

    /// Absolute value that returns `maxValue` instead of wrapping `minValue`.
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

    /// Raises `self` to `exponent` via binary exponentiation. Wraps on
    /// overflow. Negative exponents return zero (integer truncation of
    /// the would-be fraction).
    ///
    /// # Examples
    ///
    /// ```
    /// (2).pow(10);  // 1024
    /// (3).pow(4);   // 81
    /// (5).pow(-1);  // 0
    /// ```
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

    /// Greatest common divisor via Euclidean algorithm. For signed types
    /// the inputs are taken absolute first; the result is always non-negative.
    ///
    /// # Examples
    ///
    /// ```
    /// (12).gcd(8);   // 4
    /// (17).gcd(5);   // 1   (coprime)
    /// (-12).gcd(8);  // 4
    /// ```
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

    /// Least common multiple, computed as `|self| / gcd(self, other) * |other|`
    /// to avoid intermediate overflow. Returns zero if either input is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// (4).lcm(6);   // 12
    /// (3).lcm(5);   // 15
    /// (0).lcm(7);   // 0
    /// ```
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

    /// Clamps `self` into `[min, max]`. Caller is responsible for ensuring
    /// `min <= max`; otherwise the result is undefined.
    ///
    /// # Examples
    ///
    /// ```
    /// (5).clamp(0, 10);    // 5
    /// (-5).clamp(0, 10);   // 0
    /// (15).clamp(0, 10);   // 10
    /// ```
    public func clamp(min: Int32, max: Int32) -> Int32 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    /// Bitwise AND. `0b1010 & 0b1100 == 0b1000`.
    public consuming func bitwiseAnd(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_and(self.raw, other.raw)) }

    /// Bitwise OR. `0b1010 | 0b1100 == 0b1110`.
    public consuming func bitwiseOr(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_or(self.raw, other.raw)) }

    /// Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.
    public consuming func bitwiseXor(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_xor(self.raw, other.raw)) }

    /// Bitwise NOT — flips all bits. For signed types this is `-self - 1`.
    public consuming func bitwiseNot() -> Int32 { Int32(raw: lang.i32_not(self.raw)) }

    /// Left shift by `count`. Behavior is undefined when `count >= bitWidth`
    /// — pre-mask the count if you can't guarantee the bound.
    public consuming func shiftLeft(consuming by count: Int64) -> Int32 { Int32(raw: lang.i32_shl(self.raw, lang.cast_i64_i32(count.raw))) }

    /// Right shift by `count`. Arithmetic (sign-extending) for signed types,
    /// logical (zero-filling) for unsigned. Same `count` precondition as
    /// `shiftLeft`.
    public consuming func shiftRight(consuming by count: Int64) -> Int32 { Int32(raw: lang.i32_signed_shr(self.raw, lang.cast_i64_i32(count.raw))) }

    /// Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
    /// MSB re-enter at the LSB.
    public func rotateLeft(by count: Int64) -> Int32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c).bitwiseOr(self.shiftRight(by: bits - c)) }
    }

    /// Rotates bits right by `count`, modulo `bitWidth`. Mirror of
    /// `rotateLeft`.
    public func rotateRight(by count: Int64) -> Int32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c).bitwiseOr(self.shiftLeft(by: bits - c)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    /// `self += other`
    public mutating func addAssign(other: Int32) { self = self.add(other) }
    /// `self -= other`
    public mutating func subtractAssign(other: Int32) { self = self.subtract(other) }
    /// `self *= other`
    public mutating func multiplyAssign(other: Int32) { self = self.multiply(other) }
    /// `self /= other`
    public mutating func divideAssign(other: Int32) { self = self.divide(other) }
    /// `self %= other`
    public mutating func modAssign(other: Int32) { self = self.modulo(other) }
    /// `self &= other`
    public mutating func bitwiseAndAssign(other: Int32) { self = self.bitwiseAnd(other) }
    /// `self |= other`
    public mutating func bitwiseOrAssign(other: Int32) { self = self.bitwiseOr(other) }
    /// `self ^= other`
    public mutating func bitwiseXorAssign(other: Int32) { self = self.bitwiseXor(other) }
    /// `self <<= count`
    public mutating func shiftLeftAssign(by count: Int64) { self = self.shiftLeft(by: count) }
    /// `self >>= count`
    public mutating func shiftRightAssign(by count: Int64) { self = self.shiftRight(by: count) }

    // ========================================================================
    // BYTE CONVERSION
    // ========================================================================

    /// Splits this integer into 4 bytes in *native* (host) byte order.
    /// Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
    /// a fixed wire format.
    ///
    /// # Examples
    ///
    /// ```
    /// let bytes = Int32.maxValue.toBytes();   // 4 bytes, host order
    /// ```
    public func toBytes() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 4);
        let value = self;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 4 {
            result.append(ptr.offset(by: i).read());
            i = i + 1
        }
        result
    }

    /// Splits this integer into 4 bytes in big-endian order (most
    /// significant byte first — i.e. network byte order).
    public func toBytesBigEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 4);
        let value = UInt64(from: self);
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < 4 {
            let shift = (4 - 1 - i) * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(from: byteVal));
            i = i + 1
        }
        result
    }

    /// Splits this integer into 4 bytes in little-endian order (least
    /// significant byte first).
    public func toBytesLittleEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 4);
        let value = UInt64(from: self);
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < 4 {
            let shift = i * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(from: byteVal));
            i = i + 1
        }
        result
    }

    /// @name From Bytes
    /// Reassembles a `Int32` from 4 bytes in native byte order.
    /// Returns `null` if the input is not exactly 4 bytes long.
    public init[S](fromBytes fromBytes: S)? where S: Slice[UInt8] {
        let bytes = fromBytes.asSlice();
        if bytes.count != 4 {
            return null
        }
        var value = Int32.zero;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 4 {
            ptr.offset(by: i).write(bytes(unchecked: i));
            i = i + 1
        }
        self.raw = value.raw;
    }

    /// @name From Bytes Big Endian
    /// Reassembles a `Int32` from 4 bytes in big-endian order.
    /// Returns `null` if the input is not exactly 4 bytes long.
    public init[S](fromBytesBigEndian fromBytesBigEndian: S)? where S: Slice[UInt8] {
        let bytes = fromBytesBigEndian.asSlice();
        if bytes.count != 4 {
            return null
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 4 {
            let byteVal = UInt64(from: bytes(unchecked: i));
            result = (result << 8) | byteVal;
            i = i + 1
        }
        self.raw = Int32(from: result).raw;
    }

    /// @name From Bytes Little Endian
    /// Reassembles a `Int32` from 4 bytes in little-endian order.
    /// Returns `null` if the input is not exactly 4 bytes long.
    public init[S](fromBytesLittleEndian fromBytesLittleEndian: S)? where S: Slice[UInt8] {
        let bytes = fromBytesLittleEndian.asSlice();
        if bytes.count != 4 {
            return null
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 4 {
            let shift = i * 8;
            let byteVal = UInt64(from: bytes(unchecked: i));
            result = result | (byteVal << shift);
            i = i + 1
        }
        self.raw = Int32(from: result).raw;
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    /// @name Parsing
    /// Parses a base-10 integer literal, optionally prefixed with `+` or `-`.
    /// Returns `null` for an empty string, a non-digit character,
    /// or a value that does not fit in `Int32`.
    ///
    /// # Examples
    ///
    /// ```
    /// Int32(parsing: "42");    // Some(42)
    /// Int32(parsing: "-7");    // Some(-7)
    /// Int32(parsing: "abc");   // None
    /// ```
    public init(parsing string: String)? {
        let len = string.byteCount;
        if len == 0 {
            return null
        }

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = string.bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {
            isNegative = true;
            index = 1
        } else if firstByteVal == 43 {
            index = 1
        }

        if index >= len {
            return null
        }

        var result: Int64 = 0;
        let maxBeforeMultiply: Int64 = 922337203685477580;

        while index < len {
            let byte: UInt8 = string.bytes(unchecked: index);
            let byteVal = Int64(from: byte);

            if byteVal < 48 or byteVal > 57 {
                return null
            }

            let digit = byteVal - 48;

            if result > maxBeforeMultiply {
                return null
            }
            result = result * 10;

            if result > 9223372036854775807 - digit {
                return null
            }
            result = result + digit;

            index = index + 1
        }

        if isNegative {
            result = result.negate();
            if result < Int64(from: Int32.minValue) {
                return null
            }
        } else {
            if result > Int64(from: Int32.maxValue) {
                return null
            }
        }

        self.raw = Int32(from: result).raw;
    }
    /// @name Parsing with Radix
    /// Parses an integer in `radix` (base 2-36 inclusive). Letters a-z are
    /// case-insensitive and represent digit values 10-35.
    ///
    /// # Examples
    ///
    /// ```
    /// Int32(parsing: "ff", radix: 16);     // Some(255 if it fits, else None)
    /// Int32(parsing: "101010", radix: 2);  // Some(42)
    /// Int32(parsing: "z", radix: 36);      // Some(35)
    /// ```
    public init(parsing string: String, radix radix: Int64)? {
        if radix < 2 or radix > 36 {
            return null
        }

        let len = string.byteCount;
        if len == 0 {
            return null
        }

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = string.bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {
            isNegative = true;
            index = 1
        } else if firstByteVal == 43 {
            index = 1
        }

        if index >= len {
            return null
        }

        let radixU: UInt64 = UInt64(from: radix);
        let maxMagnitude: UInt64 = if isNegative {
            UInt64(from: Int32.maxValue) + 1
        } else {
            UInt64(from: Int32.maxValue)
        };

        var result: UInt64 = 0;

        while index < len {
            let byte: UInt8 = string.bytes(unchecked: index);
            let byteVal = Int64(from: byte);

            let digit: Int64 = if byteVal >= 48 and byteVal <= 57 {
                byteVal - 48
            } else if byteVal >= 65 and byteVal <= 90 {
                byteVal - 55
            } else if byteVal >= 97 and byteVal <= 122 {
                byteVal - 87
            } else {
                return null
            };

            if digit >= radix {
                return null
            }

            let digitU: UInt64 = UInt64(from: digit);
            if result > (maxMagnitude - digitU) / radixU {
                return null
            }
            result = result * radixU + digitU;
            index = index + 1
        }

        let typedResult = Int32(from: result);
        if isNegative {
            self.raw = typedResult.negate().raw
        } else {
            self.raw = typedResult.raw
        }
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    // Formattable
    /// Formats the integer directly into `writer`, honouring the supplied
    /// `FormatOptions`. Implements the `Formattable` protocol.
    ///
    /// # Examples
    ///
    /// ```
    /// (42).format();                                           // "42"
    /// (255).format(.{radix: 16});                     // "ff"
    /// (255).format(.{radix: 16, uppercase: true});    // "FF"
    /// (255).format(.{radix: 16, alternate: true});    // "0xff"
    /// (42).format(.{radix: 2, alternate: true});      // "0b101010"
    /// (42).format(.{width: .Some(5), fill: '0'});     // "00042"
    /// (-42).format(.{sign: .Always});                 // "-42"
    /// ```
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        var radix: Int64 = options.radix;
        if radix < 2 or radix > 36 {
            radix = 10
        }

        // Build digits in reverse order (need random access to reverse)
        var digits = String();
        if n == Int32.zero {
            digits.appendByte(48)
        } else {
            let radixVal: Int32 = Int32(from: radix);
            while n != Int32.zero {
                let digit: Int32 = n % radixVal;
                let digitVal: Int64 = Int64(from: digit);
                let charCode: Int64 = if digitVal < 10 {
                    digitVal + 48
                } else if options.uppercase {
                    digitVal - 10 + 65
                } else {
                    digitVal - 10 + 97
                };
                digits.appendByte(UInt8(from: charCode));
                n = n / radixVal
            }
        }

        // Build content: sign + prefix + reversed digits
        var result = String();

        if isNegative {
            result.append(char: '-')
        } else if options.sign == .Always {
            result.append(char: '+')
        } else if options.sign == .Space {
            result.append(char: ' ')
        }

        if options.alternate {
            if radix == 2 {
                result.append("0b")
            } else if radix == 8 {
                result.append("0o")
            } else if radix == 16 {
                result.append("0x")
            }
        }

        var i = digits.byteCount - 1;
        while i >= 0 {
            result.appendByte(digits.bytes(unchecked: i));
            i = i - 1
        }

        _writePadded(into: writer, result, options)
    }}

