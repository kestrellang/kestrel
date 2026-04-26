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

/// A 64-bit signed integer.
///
/// Int64 is the 64-bit member of the integer family. The same surface
/// area is provided across all widths; switch widths to trade range for memory
/// or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
/// `*Checked` variants for overflow detection or `*Saturating` to clamp to
/// `minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
/// `lang.i64` so it can cross C boundaries unchanged.
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
/// A single `lang.i64` field. No padding, no headers — bit-identical
/// to the corresponding C type.
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
    /// The underlying primitive `lang.i64` value. Exposed for FFI
    /// and intrinsic use; prefer the typed surface for everything else.
    public var raw: lang.i64

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    /// The additive identity, `0`.
    public static var zero: Int64 { Int64(intLiteral: 0) }

    /// The multiplicative identity, `1`.
    public static var one: Int64 { Int64(intLiteral: 1) }

    /// The smallest representable value.
    /// This is -2^63 (-9_223_372_036_854_775_808).
    /// Note that for signed types `minValue.negate()` overflows back to
    /// itself; use `negateChecked()` if you need to detect that.
    public static var minValue: Int64 { Int64(raw: lang.i64_shl(1, 63)) }

    /// The largest representable value.
    /// This is 2^63 - 1 (9_223_372_036_854_775_807).
    public static var maxValue: Int64 { Int64(intLiteral: 9223372036854775807) }

    /// The width in bits (64). Useful for shift bounds and bit-walks.
    public static var bitWidth: Int64 { Int64(intLiteral: 64) }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// @name Int Literal
    /// Compiler-emitted bridge that turns an integer literal into a Int64.
    ///
    /// You will rarely call this directly — write the literal and let the
    /// `ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
    /// 64 bits the literal is truncated with `lang.cast_i64_i64`.
    ///
    /// # Examples
    ///
    /// ```
    /// let n: Int64 = 42;            // implicit
    /// let m = Int64(intLiteral: 42);  // explicit
    /// ```
    public init(intLiteral value: lang.i64) {
        self.raw = value
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
    /// Wraps an existing `lang.i64` without conversion. Internal
    /// constructor used by intrinsics; not part of the public API.
    init(raw value: lang.i64) {
        self.raw = value
    }

    /// @name From Integer
    /// Converts from `Int8`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: Int8) { self.raw = lang.cast_i8_i64(other.raw) }
    /// @name From Integer
    /// Converts from `Int16`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: Int16) { self.raw = lang.cast_i16_i64(other.raw) }
    /// @name From Integer
    /// Converts from `Int32`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: Int32) { self.raw = lang.cast_i32_i64(other.raw) }
    /// @name From Integer
    /// Converts from `UInt8`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt8) { self.raw = lang.cast_u8_i64(other.raw) }
    /// @name From Integer
    /// Converts from `UInt16`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt16) { self.raw = lang.cast_u16_i64(other.raw) }
    /// @name From Integer
    /// Converts from `UInt32`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt32) { self.raw = lang.cast_u32_i64(other.raw) }
    /// @name From Integer
    /// Converts from `UInt64`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt64) { self.raw = other.raw }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Sign as a `Int64`: `-1`, `0`, or `1`.
    public var sign: Int64 { get {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { Int64(intLiteral: lang.i64_neg(1)) }
        else if Bool(boolLiteral: lang.i64_eq(self.raw, 0)) { Int64.zero }
        else { Int64.one }
    }}

    /// True when `self > 0`.
    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i64_signed_gt(self.raw, 0))
    }}

    /// True when `self < 0`.
    public var isNegative: Bool { get {
        Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0))
    }}

    /// True when `self == 0`.
    public var isZero: Bool { get {
        Bool(boolLiteral: lang.i64_eq(self.raw, 0))
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
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 1)) { false }
        else { Bool(boolLiteral: lang.i64_eq(lang.i64_and(self.raw, lang.i64_sub(self.raw, 1)), 0)) }
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
        Int64(raw: lang.i64_popcount(self.raw))
    }}

    /// Complement of `countOnes`: equal to `bitWidth - countOnes`.
    public var countZeros: Int64 { get {
        Int64(intLiteral: 64) - self.countOnes
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
        Int64(raw: lang.i64_clz(self.raw))
    }}

    /// Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
    /// values; returns `bitWidth` for zero. Useful for finding the largest
    /// power of two dividing the value.
    public var trailingZeros: Int64 { get {
        Int64(raw: lang.i64_ctz(self.raw))
    }}

    /// Value with its byte order reversed. Use to convert between big- and
    /// little-endian; lowered to a `bswap` intrinsic.
    public var byteSwapped: Int64 { get {
        Int64(raw: lang.i64_bswap(self.raw))
    }}

    // ========================================================================
    // COMPARISON
    // ========================================================================

    /// Bit-for-bit equality. Backs the `==` operator.
    ///
    /// # Examples
    ///
    /// ```
    /// (42).equals(other: 42);  // true
    /// 42 == 42;                // true
    /// ```
    public func equals(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    /// Pattern-matching hook for `Matchable`. Identical to `equals`.
    public func matches(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    /// Three-way comparison returning an `Ordering`. Signed types compare
    /// using two's-complement ordering; unsigned types use natural ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// (1).compare(other: 2);   // .Less
    /// (2).compare(other: 2);   // .Equal
    /// (3).compare(other: 2);   // .Greater
    /// ```
    public func compare(other: Int64) -> Ordering {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i64_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
    /// integer ranges.
    public func successor() -> Int64 { self.add(Int64.one) }

    /// Predecessor — `self - 1`. Wraps at `minValue`.
    public func predecessor() -> Int64 { self.subtract(Int64.one) }

    /// Builds a half-open range `self..<end`. Sugar for the `..<` operator.
    public func exclusiveRange(to end: Int64) -> Range[Int64] {
        Range[Int64](self, end)
    }

    /// Builds a closed range `self..=end`. Sugar for the `..=` operator.
    public func inclusiveRange(to end: Int64) -> ClosedRange[Int64] {
        ClosedRange[Int64](self, end)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
    /// only within a single process — do not persist hashes across builds.
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

    /// `self + other`, wrapping on overflow. Use `addChecked` to detect or
    /// `addSaturating` to clamp.
    public func add(other: Int64) -> Int64 { Int64(raw: lang.i64_add(self.raw, other.raw)) }

    /// `self - other`, wrapping on overflow.
    public func subtract(other: Int64) -> Int64 { Int64(raw: lang.i64_sub(self.raw, other.raw)) }

    /// `self * other`, wrapping on overflow.
    public func multiply(other: Int64) -> Int64 { Int64(raw: lang.i64_mul(self.raw, other.raw)) }

    /// Truncating integer division (`self / other`). For signed types,
    /// `minValue / -1` wraps; use `divideChecked` to detect.
    ///
    /// # Errors
    ///
    /// Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
    /// process aborts before producing a result).
    public func divide(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_div(self.raw, other.raw)) }

    /// `self % other` — truncated remainder; the result has the sign of
    /// `self` for signed types.
    ///
    /// # Errors
    ///
    /// Traps on division by zero, like `divide`.
    public func modulo(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_rem(self.raw, other.raw)) }

    public func negate() -> Int64 { Int64(raw: lang.i64_neg(self.raw)) }
    public func abs() -> Int64 { if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    /// Wrapping addition that returns `None` instead of overflowing.
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

    /// Wrapping subtraction that returns `None` instead of overflowing.
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

    /// Wrapping multiplication that returns `None` instead of overflowing.
    /// Implemented by multiplying then dividing back; replace with an
    /// overflow-detecting intrinsic when one is available.
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

    /// Division that returns `None` for divide-by-zero or for the
    /// `minValue / -1` overflow case.
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

    /// Negation that returns `None` for `minValue` (whose negation overflows).
    public func negateChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.negate())
    }

    /// Absolute value that returns `None` for `minValue` (whose absolute
    /// value overflows).
    public func absChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.abs())
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    /// Addition that clamps to `maxValue`/`minValue` instead of wrapping.
    public func addSaturating(other: Int64) -> Int64 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isPositive { Int64.maxValue } else { Int64.minValue }
        }
    }

    /// Subtraction that clamps to `maxValue`/`minValue` instead of wrapping.
    public func subtractSaturating(other: Int64) -> Int64 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => if other.isNegative { Int64.maxValue } else { Int64.minValue }
        }
    }

    /// Multiplication that clamps to `maxValue`/`minValue` instead of wrapping.
    /// The clamp direction follows the algebraic sign of the would-be result.
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

    /// Negation that returns `maxValue` instead of wrapping `minValue`.
    public func negateSaturating() -> Int64 {
        if self == Int64.minValue {
            Int64.maxValue
        } else {
            self.negate()
        }
    }

    /// Absolute value that returns `maxValue` instead of wrapping `minValue`.
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

    /// Clamps `self` into `[min, max]`. Caller is responsible for ensuring
    /// `min <= max`; otherwise the result is undefined.
    ///
    /// # Examples
    ///
    /// ```
    /// (5).clamp(min: 0, max: 10);    // 5
    /// (-5).clamp(min: 0, max: 10);   // 0
    /// (15).clamp(min: 0, max: 10);   // 10
    /// ```
    public func clamp(min: Int64, max: Int64) -> Int64 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    /// Bitwise AND. `0b1010 & 0b1100 == 0b1000`.
    public func bitwiseAnd(other: Int64) -> Int64 { Int64(raw: lang.i64_and(self.raw, other.raw)) }

    /// Bitwise OR. `0b1010 | 0b1100 == 0b1110`.
    public func bitwiseOr(other: Int64) -> Int64 { Int64(raw: lang.i64_or(self.raw, other.raw)) }

    /// Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.
    public func bitwiseXor(other: Int64) -> Int64 { Int64(raw: lang.i64_xor(self.raw, other.raw)) }

    /// Bitwise NOT — flips all bits. For signed types this is `-self - 1`.
    public func bitwiseNot() -> Int64 { Int64(raw: lang.i64_not(self.raw)) }

    /// Left shift by `count`. Behavior is undefined when `count >= bitWidth`
    /// — pre-mask the count if you can't guarantee the bound.
    public func shiftLeft(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_shl(self.raw, count)) }

    /// Right shift by `count`. Arithmetic (sign-extending) for signed types,
    /// logical (zero-filling) for unsigned. Same `count` precondition as
    /// `shiftLeft`.
    public func shiftRight(by count: lang.i64) -> Int64 { Int64(raw: lang.i64_signed_shr(self.raw, count)) }

    /// Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
    /// MSB re-enter at the LSB.
    public func rotateLeft(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c.raw).bitwiseOr(self.shiftRight(by: (bits - c).raw)) }
    }

    /// Rotates bits right by `count`, modulo `bitWidth`. Mirror of
    /// `rotateLeft`.
    public func rotateRight(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c.raw).bitwiseOr(self.shiftLeft(by: (bits - c).raw)) }
    }

    // ========================================================================
    // COMPOUND ASSIGNMENT
    // ========================================================================

    /// `self += other`
    public mutating func addAssign(other: Int64) { self = self.add(other) }
    /// `self -= other`
    public mutating func subtractAssign(other: Int64) { self = self.subtract(other) }
    /// `self *= other`
    public mutating func multiplyAssign(other: Int64) { self = self.multiply(other) }
    /// `self /= other`
    public mutating func divideAssign(other: Int64) { self = self.divide(other) }
    /// `self %= other`
    public mutating func modAssign(other: Int64) { self = self.modulo(other) }
    /// `self &= other`
    public mutating func bitwiseAndAssign(other: Int64) { self = self.bitwiseAnd(other) }
    /// `self |= other`
    public mutating func bitwiseOrAssign(other: Int64) { self = self.bitwiseOr(other) }
    /// `self ^= other`
    public mutating func bitwiseXorAssign(other: Int64) { self = self.bitwiseXor(other) }
    /// `self <<= count`
    public mutating func shiftLeftAssign(by count: lang.i64) { self = self.shiftLeft(by: count) }
    /// `self >>= count`
    public mutating func shiftRightAssign(by count: lang.i64) { self = self.shiftRight(by: count) }

    // ========================================================================
    // BYTE CONVERSION
    // ========================================================================

    /// Splits this integer into 8 bytes in *native* (host) byte order. Use
    /// `toBytesBigEndian` / `toBytesLittleEndian` when serialising for a
    /// fixed wire format.
    ///
    /// # Examples
    ///
    /// ```
    /// let bytes = (0x0102030405060708).toBytes();
    /// // little-endian host: [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]
    /// // big-endian host:    [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    /// ```
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

    /// Splits this integer into 8 bytes in big-endian order (most significant
    /// byte first — i.e. network byte order).
    ///
    /// # Examples
    ///
    /// ```
    /// (0x0102030405060708).toBytesBigEndian();
    /// // [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    /// ```
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

    /// Splits this integer into 8 bytes in little-endian order (least
    /// significant byte first).
    ///
    /// # Examples
    ///
    /// ```
    /// (0x0102030405060708).toBytesLittleEndian();
    /// // [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]
    /// ```
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

    /// Reassembles an `Int64` from 8 bytes in native (host) byte order.
    /// Returns `None` if the input is not exactly 8 bytes long.
    ///
    /// # Examples
    ///
    /// ```
    /// Int64.fromBytes(bytes: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    /// ```
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

    /// Reassembles an `Int64` from 8 bytes in big-endian order. Returns
    /// `None` if the input is not exactly 8 bytes long.
    ///
    /// # Examples
    ///
    /// ```
    /// Int64.fromBytesBigEndian(bytes: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    /// ```
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

    /// Reassembles an `Int64` from 8 bytes in little-endian order. Returns
    /// `None` if the input is not exactly 8 bytes long.
    ///
    /// # Examples
    ///
    /// ```
    /// Int64.fromBytesLittleEndian(bytes: [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]);
    /// ```
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

    /// Parses a base-10 integer literal, optionally prefixed with `+` or
    /// `-`. Returns `None` for an empty string, a non-digit character,
    /// or a value that does not fit in `Int64`.
    ///
    /// # Examples
    ///
    /// ```
    /// Int64.parse(string: "42");    // Some(42)
    /// Int64.parse(string: "-7");    // Some(-7)
    /// Int64.parse(string: "abc");   // None
    /// Int64.parse(string: "");      // None
    /// ```
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
    /// Parses an integer in `radix` (base 2–36 inclusive). Letters a–z are
    /// case-insensitive and represent digit values 10–35. Returns `None`
    /// for an out-of-range radix, an empty string, an unrecognised digit,
    /// or a value that overflows `Int64`.
    ///
    /// # Examples
    ///
    /// ```
    /// Int64.parse(string: "ff", radix: 16);     // Some(255)
    /// Int64.parse(string: "FF", radix: 16);     // Some(255)
    /// Int64.parse(string: "101010", radix: 2);  // Some(42)
    /// Int64.parse(string: "z", radix: 36);      // Some(35)
    /// ```
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
    /// Renders the integer to a `String`, honouring the supplied
    /// `FormatOptions`. Implements the `Formattable` protocol.
    ///
    /// Recognised options:
    /// - `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
    /// - `width` — minimum output width; shorter values are padded.
    /// - `fill` / `alignment` — padding character and side.
    /// - `sign` — `.Negative` (default), `.Always`, or `.Space`.
    /// - `uppercase` — uppercase hex digits.
    /// - `alternate` — emit the `0b` / `0o` / `0x` prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// (42).format();                                           // "42"
    /// (255).format(options: .{radix: 16});                     // "ff"
    /// (255).format(options: .{radix: 16, uppercase: true});    // "FF"
    /// (255).format(options: .{radix: 16, alternate: true});    // "0xff"
    /// (42).format(options: .{radix: 2, alternate: true});      // "0b101010"
    /// (42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
    /// (-42).format(options: .{sign: .Always});                 // "-42"
    /// ```
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        var n = self;
        let isNegative = n < 0;
        if isNegative {
            n = n.negate()
        }

        // Get radix (default 10)
        var radix: Int64 = options.radix;
        if radix < 2 or radix > 36 {
            radix = 10
        }

        // Build digits in reverse order
        var digits = String();
        if n == Int64.zero {
            digits.appendByte(48)  // '0'
        } else {
            let radixVal: Int64 = radix;
            while n != Int64.zero {
                let digit: Int64 = n % radixVal;
                let digitVal: Int64 = digit;
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

        // Add sign prefix
        if isNegative {
            result.appendByte(45)  // '-'
        } else if options.sign == .Always {
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

/// Platform-sized signed integer — currently always `Int64`.
public type Int = Int64
