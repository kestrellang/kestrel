// Int32 - 32-bit signed integer
// GENERATED FILE — never edit it directly. Edit lang/std/numeric/integer.ks.template
// (or generate.py) and rerun `python3 generate.py` from lang/std/numeric.

module std.numeric

import std.ffi.(FFISafe)
import std.core.(
    Equatable, Comparable, Ordering, Bool, Matchable, Hashable, Hasher,
    Addable, Subtractable, Multipliable, Divisible, Remainder, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift,
    AddAssign, SubtractAssign, MultiplyAssign, DivideAssign, RemainderAssign,
    BitwiseAndAssign, BitwiseOrAssign, BitwiseXorAssign, LeftShiftAssign, RightShiftAssign,
    ExpressibleByIntLiteral, Convertible, LossyConvertible, Defaultable,
    RangeConstructible, ClosedRangeConstructible, Range, ClosedRange,
    RangeFromConstructible, RangeUpToConstructible, RangeThroughConstructible,
    RangeFrom, RangeUpTo, RangeThrough
)
import std.text.(String, Str, StringBuilder, Formatter, Formattable, FormatOptions, _writePadded)
import std.memory.(UnsafeSlice, Pointer)
import std.collections.(Sliceable, Slice)
import std.numeric.(UInt8, Int64, UInt64)

/// A 32-bit signed integer.
///
/// Int32 is the 32-bit member of the integer family. The same surface
/// area is provided across all widths; switch widths to trade range for memory
/// or to match an FFI ABI. Arithmetic traps on overflow in unoptimized builds and wraps in
/// optimized ones (`--overflow-checks` overrides). Use the `*Wrapping`,
/// `*Checked`, `*Saturating` or `*Overflowing` variants to pick a behaviour
/// that holds in every build. The type is `FFISafe` and lays out as a single
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
    Remainder,
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
    RemainderAssign,
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
    Convertible[UInt64],
    LossyConvertible[Int64],
    LossyConvertible[UInt32],
    LossyConvertible[UInt64]
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
    /// Wraps an existing `lang.i32` without conversion. The FFI
    /// counterpart of `raw`, for values coming back from intrinsics or C.
    public init(raw value: lang.i32) {
        self.raw = value
    }

    /// @name From Integer
    /// Converts from `Int8`. Lossless: every `Int8` fits in `Int32`.
    public init(from other: Int8) { self.raw = lang.cast_i8_i32(other.raw) }
    /// @name From Integer
    /// Converts from `Int16`. Lossless: every `Int16` fits in `Int32`.
    public init(from other: Int16) { self.raw = lang.cast_i16_i32(other.raw) }
    /// @name From Integer
    /// Converts from `Int64`, trapping when the value doesn't fit in
    /// `Int32`. Use `init(truncating:)` or `init(clamping:)` when
    /// out-of-range values are expected.
    ///
    /// # Errors
    ///
    /// Panics when `other` is outside `Int32.minValue...Int32.maxValue`.
    public init(from other: Int64) {
        if other < Int64(from: Int32.minValue) or other > Int64(from: Int32.maxValue) {
            fatalError("integer conversion overflow: value doesn't fit in Int32")
        }
        self.raw = lang.cast_i64_i32(other.raw)
    }
    /// @name Truncating
    /// Converts from `Int64` keeping only the low bits, reinterpreted as
    /// `Int32`. Never traps.
    public init(truncating other: Int64) { self.raw = lang.cast_i64_i32(other.raw) }
    /// @name Clamping
    /// Converts from `Int64`, saturating to `Int32.minValue` / `Int32.maxValue`
    /// when the value doesn't fit.
    public init(clamping other: Int64) {
        if other < Int64(from: Int32.minValue) {
            self.raw = Int32.minValue.raw
        } else if other > Int64(from: Int32.maxValue) {
            self.raw = Int32.maxValue.raw
        } else {
            self.raw = lang.cast_i64_i32(other.raw)
        }
    }
    /// @name From Integer
    /// Converts from `UInt8`. Lossless: every `UInt8` fits in `Int32`.
    public init(from other: UInt8) { self.raw = lang.cast_u8_i32(other.raw) }
    /// @name From Integer
    /// Converts from `UInt16`. Lossless: every `UInt16` fits in `Int32`.
    public init(from other: UInt16) { self.raw = lang.cast_u16_i32(other.raw) }
    /// @name From Integer
    /// Converts from `UInt32`, trapping when the value doesn't fit in
    /// `Int32`. Use `init(truncating:)` or `init(clamping:)` when
    /// out-of-range values are expected.
    ///
    /// # Errors
    ///
    /// Panics when `other` is outside `Int32.minValue...Int32.maxValue`.
    public init(from other: UInt32) {
        if other > UInt32(truncating: Int32.maxValue) {
            fatalError("integer conversion overflow: value doesn't fit in Int32")
        }
        self.raw = other.raw
    }
    /// @name Truncating
    /// Converts from `UInt32` keeping only the low bits, reinterpreted as
    /// `Int32`. Never traps.
    public init(truncating other: UInt32) { self.raw = other.raw }
    /// @name Clamping
    /// Converts from `UInt32`, saturating to `Int32.minValue` / `Int32.maxValue`
    /// when the value doesn't fit.
    public init(clamping other: UInt32) {
        if other > UInt32(truncating: Int32.maxValue) {
            self.raw = Int32.maxValue.raw
        } else {
            self.raw = other.raw
        }
    }
    /// @name Bit Pattern
    /// Reinterprets the bits of a same-width `UInt32` as `Int32`. Never traps.
    public init(bitPattern other: UInt32) { self.raw = other.raw }
    /// @name From Integer
    /// Converts from `UInt64`, trapping when the value doesn't fit in
    /// `Int32`. Use `init(truncating:)` or `init(clamping:)` when
    /// out-of-range values are expected.
    ///
    /// # Errors
    ///
    /// Panics when `other` is outside `Int32.minValue...Int32.maxValue`.
    public init(from other: UInt64) {
        if other > UInt64(truncating: Int32.maxValue) {
            fatalError("integer conversion overflow: value doesn't fit in Int32")
        }
        self.raw = lang.cast_u64_i32(other.raw)
    }
    /// @name Truncating
    /// Converts from `UInt64` keeping only the low bits, reinterpreted as
    /// `Int32`. Never traps.
    public init(truncating other: UInt64) { self.raw = lang.cast_u64_i32(other.raw) }
    /// @name Clamping
    /// Converts from `UInt64`, saturating to `Int32.minValue` / `Int32.maxValue`
    /// when the value doesn't fit.
    public init(clamping other: UInt64) {
        if other > UInt64(truncating: Int32.maxValue) {
            self.raw = Int32.maxValue.raw
        } else {
            self.raw = lang.cast_u64_i32(other.raw)
        }
    }

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

    /// Successor — `self + 1`. Overflows at `maxValue`, like `add`. Used by `for-in` over
    /// integer ranges.
    public func successor() -> Int32 { self.add(Int32.one) }

    /// Predecessor — `self - 1`. Overflows at `minValue`, like `subtract`.
    public func predecessor() -> Int32 { self.subtract(Int32.one) }

    /// Number of `successor()` steps from `self` to `other` — `other - self`
    /// widened to `Int64` (negative when `other < self`). `O(1)`; lets closed
    /// ranges iterate with a counter instead of a "finished" flag.
    public func distance(to other: Int32) -> Int64 {
        Int64(raw: lang.i64_sub(lang.cast_i32_i64(other.raw), lang.cast_i32_i64(self.raw)))
    }

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

    /// Feeds this value into `hasher` as one 64-bit word
    /// (`write(uint64:)`), which `DefaultHasher` mixes in a single round.
    /// Don't persist hashes across builds.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(uint64: UInt64(truncating: self))
    }

    // ========================================================================
    // ASSOCIATED TYPE BINDINGS
    // ========================================================================

    type Addable.Output = Int32
    type Subtractable.Output = Int32
    type Multipliable.Output = Int32
    type Divisible.Output = Int32
    type Remainder.Output = Int32
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
    // ARITHMETIC (Overflow-checked)
    // ========================================================================

    /// `self + other`. Traps on overflow when overflow checks are on; use `addWrapping`, `addChecked`,
    /// `addSaturating` or `addOverflowing` when overflow is expected.
    ///
    /// # Errors
    ///
    /// Panics when the result doesn't fit in `Int32` and overflow checks are
    /// on (the default at `-O0`). With checks off it wraps.
    public consuming func add(consuming other: Int32) -> Int32 {
        if Bool(boolLiteral: lang.i1_and(lang.overflow_checks(), lang.i32_signed_add_overflows(self.raw, other.raw))) {
            fatalError("arithmetic overflow: Int32 +")
        }
        Int32(raw: lang.i32_add(self.raw, other.raw))
    }

    /// `self + other`, wrapping around on overflow (two's complement).
    /// For hashes, checksums and RNGs where wrap-around is the intent.
    public func addWrapping(other: Int32) -> Int32 { Int32(raw: lang.i32_add(self.raw, other.raw)) }

    /// `self + other` plus whether it overflowed. The value is the
    /// wrapped result, as from `addWrapping`.
    public func addOverflowing(other: Int32) -> (Int32, Bool) {
        (Int32(raw: lang.i32_add(self.raw, other.raw)), Bool(boolLiteral: lang.i32_signed_add_overflows(self.raw, other.raw)))
    }

    /// `self - other`. Traps on overflow when overflow checks are on; use `subtractWrapping`, `subtractChecked`,
    /// `subtractSaturating` or `subtractOverflowing` when overflow is expected.
    ///
    /// # Errors
    ///
    /// Panics when the result doesn't fit in `Int32` and overflow checks are
    /// on (the default at `-O0`). With checks off it wraps.
    public consuming func subtract(consuming other: Int32) -> Int32 {
        if Bool(boolLiteral: lang.i1_and(lang.overflow_checks(), lang.i32_signed_sub_overflows(self.raw, other.raw))) {
            fatalError("arithmetic overflow: Int32 -")
        }
        Int32(raw: lang.i32_sub(self.raw, other.raw))
    }

    /// `self - other`, wrapping around on overflow (two's complement).
    /// For hashes, checksums and RNGs where wrap-around is the intent.
    public func subtractWrapping(other: Int32) -> Int32 { Int32(raw: lang.i32_sub(self.raw, other.raw)) }

    /// `self - other` plus whether it overflowed. The value is the
    /// wrapped result, as from `subtractWrapping`.
    public func subtractOverflowing(other: Int32) -> (Int32, Bool) {
        (Int32(raw: lang.i32_sub(self.raw, other.raw)), Bool(boolLiteral: lang.i32_signed_sub_overflows(self.raw, other.raw)))
    }

    /// `self * other`. Traps on overflow when overflow checks are on; use `multiplyWrapping`, `multiplyChecked`,
    /// `multiplySaturating` or `multiplyOverflowing` when overflow is expected.
    ///
    /// # Errors
    ///
    /// Panics when the result doesn't fit in `Int32` and overflow checks are
    /// on (the default at `-O0`). With checks off it wraps.
    public consuming func multiply(consuming other: Int32) -> Int32 {
        if Bool(boolLiteral: lang.i1_and(lang.overflow_checks(), lang.i32_signed_mul_overflows(self.raw, other.raw))) {
            fatalError("arithmetic overflow: Int32 *")
        }
        Int32(raw: lang.i32_mul(self.raw, other.raw))
    }

    /// `self * other`, wrapping around on overflow (two's complement).
    /// For hashes, checksums and RNGs where wrap-around is the intent.
    public func multiplyWrapping(other: Int32) -> Int32 { Int32(raw: lang.i32_mul(self.raw, other.raw)) }

    /// `self * other` plus whether it overflowed. The value is the
    /// wrapped result, as from `multiplyWrapping`.
    public func multiplyOverflowing(other: Int32) -> (Int32, Bool) {
        (Int32(raw: lang.i32_mul(self.raw, other.raw)), Bool(boolLiteral: lang.i32_signed_mul_overflows(self.raw, other.raw)))
    }

    /// The full double-width product `self * other` as `(low, high)`
    /// halves: `low` is the wrapped product (as from `multiplyWrapping`),
    /// `high` the bits above it. Compiles to one widening multiply. For
    /// hashes, RNGs and bignum arithmetic.
    public func multiplyFull(other: Int32) -> (Int32, Int32) {
        (Int32(raw: lang.i32_mul(self.raw, other.raw)), Int32(raw: lang.i32_signed_mul_high(self.raw, other.raw)))
    }

    /// Truncating integer division (`self / other`). For signed types,
    /// `minValue / -1` traps when overflow checks are on (and wraps
    /// when off); use `divideWrapping` or `divideChecked`.
    ///
    /// # Errors
    ///
    /// Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
    /// process aborts before producing a result).
    public consuming func divide(consuming other: Int32) -> Int32 {
        if Bool(boolLiteral: lang.overflow_checks()) and self == Int32.minValue and other == -1 {
            fatalError("arithmetic overflow: Int32.minValue / -1")
        }
        Int32(raw: lang.i32_signed_div(self.raw, other.raw))
    }

    /// `self / other`, with `minValue / -1` wrapping to `minValue`.
    /// Still traps on division by zero.
    public func divideWrapping(other: Int32) -> Int32 { Int32(raw: lang.i32_signed_div(self.raw, other.raw)) }

    /// `self % other` — truncated remainder; the result has the sign of
    /// `self` for signed types.
    ///
    /// # Errors
    ///
    /// Traps on division by zero, like `divide`.
    public consuming func remainder(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_signed_rem(self.raw, other.raw)) }

    /// Euclidean modulo: the result is always in `0..<|other|`, whatever
    /// the signs. Differs from `%` (`remainder`) only when `self` is
    /// negative: `(-7) % 3` is `-1`, `(-7).modulo(by: 3)` is `2`.
    ///
    /// # Errors
    ///
    /// Traps on division by zero, like `remainder`.
    ///
    /// # Examples
    ///
    /// ```
    /// (-7).modulo(by: 3);   // 2
    /// (7).modulo(by: -3);   // 1
    /// (-7).modulo(by: -3);  // 2
    /// ```
    public func modulo(by other: Int32) -> Int32 {
        let r = self.remainder(other);
        if r >= 0 {
            return r
        }
        // |r| < |other|, so neither branch overflows (even for minValue).
        if other < 0 { r - other } else { r + other }
    }

    /// `self / other` without the divide-by-zero and `minValue / -1` guards —
    /// the bare hardware divide. Faster in hot loops, but **undefined
    /// behaviour** if `other == 0` or (for signed types) `self == minValue and
    /// other == -1`. The caller must guarantee a valid divisor. Prefer
    /// `divide` everywhere correctness matters; this is the `arr(unchecked:)`
    /// of arithmetic.
    ///
    /// # Safety
    ///
    /// UB when `other == 0`, or signed `minValue / -1`.
    public consuming func divideUnchecked(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_signed_div_unchecked(self.raw, other.raw)) }

    /// `self % other` without the divide-by-zero and `minValue % -1` guards.
    /// Same safety contract as `divideUnchecked`.
    ///
    /// # Safety
    ///
    /// UB when `other == 0`, or signed `minValue % -1`.
    public consuming func remainderUnchecked(consuming other: Int32) -> Int32 { Int32(raw: lang.i32_signed_rem_unchecked(self.raw, other.raw)) }

    /// Two's-complement negation. Traps at the minimum value, which has
    /// no positive counterpart, when overflow checks are on (wraps when off); use `negateWrapping`, `negateChecked` or
    /// `negateSaturating` there.
    public consuming func negate() -> Int32 {
        if Bool(boolLiteral: lang.overflow_checks()) and self == Int32.minValue {
            fatalError("arithmetic overflow: -Int32.minValue")
        }
        Int32(raw: lang.i32_neg(self.raw))
    }

    /// `-self`, with `-minValue` wrapping back to `minValue`.
    public func negateWrapping() -> Int32 { Int32(raw: lang.i32_neg(self.raw)) }
    /// Absolute value. Traps at the minimum value when overflow checks are on; use `absWrapping`,
    /// `absChecked` or `absSaturating` there.
    public func abs() -> Int32 { if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    /// `|self|`, with `minValue.absWrapping()` wrapping to `minValue`.
    public func absWrapping() -> Int32 { if Bool(boolLiteral: lang.i32_signed_lt(self.raw, 0)) { self.negateWrapping() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    /// `self + other`, or `None` on overflow.
    public func addChecked(other: Int32) -> Int32? {
        if Bool(boolLiteral: lang.i32_signed_add_overflows(self.raw, other.raw)) { return .None };
        .Some(self.addWrapping(other))
    }

    /// `self - other`, or `None` on overflow.
    public func subtractChecked(other: Int32) -> Int32? {
        if Bool(boolLiteral: lang.i32_signed_sub_overflows(self.raw, other.raw)) { return .None };
        .Some(self.subtractWrapping(other))
    }

    /// `self * other`, or `None` on overflow.
    public func multiplyChecked(other: Int32) -> Int32? {
        if Bool(boolLiteral: lang.i32_signed_mul_overflows(self.raw, other.raw)) { return .None };
        .Some(self.multiplyWrapping(other))
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
        .Some(self.divideWrapping(other))
    }

    /// Negation that returns `None` for `minValue` (whose negation overflows).
    public func negateChecked() -> Int32? {
        if self == Int32.minValue {
            return .None
        };
        .Some(self.negateWrapping())
    }

    /// Absolute value that returns `None` for `minValue` (whose absolute
    /// value overflows).
    public func absChecked() -> Int32? {
        if self == Int32.minValue {
            return .None
        };
        .Some(self.absWrapping())
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

    /// Raises `self` to `exponent` via binary exponentiation.
    ///
    /// # Errors
    ///
    /// Panics on a negative exponent, and on overflow when overflow checks
    /// are on (wraps when off); see `powChecked`,
    /// `powWrapping` and `powSaturating`.
    ///
    /// # Examples
    ///
    /// ```
    /// (2).pow(10);  // 1024
    /// (3).pow(4);   // 81
    /// ```
    public func pow(exponent: Int64) -> Int32 {
        if Bool(boolLiteral: lang.i1_not(lang.overflow_checks())) {
            return self.powWrapping(exponent)
        }
        match self.powChecked(exponent) {
            .Some(value) => value,
            .None => fatalError("arithmetic overflow: Int32.pow")
        }
    }

    /// `self` raised to `exponent`, or `None` on overflow or a negative
    /// exponent.
    public func powChecked(exponent: Int64) -> Int32? {
        if exponent < 0 {
            return .None
        }
        var result: Int32 = 1;
        var base = self;
        var exp = exponent;
        while exp > 0 {
            if exp % 2 == 1 {
                let next = result.multiplyChecked(base);
                if next.isNone { return .None }
                result = next.unwrap()
            }
            exp = exp / 2;
            // Square only when another bit remains, so the last step can't
            // overflow a result that fits.
            if exp > 0 {
                let squared = base.multiplyChecked(base);
                if squared.isNone { return .None }
                base = squared.unwrap()
            }
        }
        .Some(result)
    }

    /// `self` raised to `exponent`, wrapping on overflow. A negative
    /// exponent traps.
    public func powWrapping(exponent: Int64) -> Int32 {
        if exponent < 0 {
            fatalError("pow: negative exponent")
        }
        var result: Int32 = 1;
        var base = self;
        var exp = exponent;
        while exp > 0 {
            if exp % 2 == 1 {
                result = result.multiplyWrapping(base)
            }
            base = base.multiplyWrapping(base);
            exp = exp / 2
        }
        result
    }

    /// `self` raised to `exponent`, clamped to `minValue`/`maxValue` on
    /// overflow. A negative exponent traps.
    public func powSaturating(exponent: Int64) -> Int32 {
        if exponent < 0 {
            fatalError("pow: negative exponent")
        }
        match self.powChecked(exponent) {
            .Some(value) => value,
            .None => if self < 0 and exponent % 2 == 1 { Int32.minValue } else { Int32.maxValue }
        }
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
        var a = self.absWrapping();
        var b = other.absWrapping();
        // `minValue.abs()` wraps to `minValue`, so a remainder can come out
        // negative: keep each remainder's magnitude so the result is
        // non-negative (only `gcd(minValue, 0)` = 2^(n-1) is unrepresentable).
        while b != Int32.zero {
            let t = b;
            b = a.remainder(b).absWrapping();
            a = t
        };
        a.absWrapping()
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

    /// Clamps `self` into `[min, max]`.
    ///
    /// # Errors
    ///
    /// Panics with `"clamp: min must be <= max"` when `min > max`.
    ///
    /// # Examples
    ///
    /// ```
    /// (5).clamp(0, 10);    // 5
    /// (-5).clamp(0, 10);   // 0
    /// (15).clamp(0, 10);   // 10
    /// ```
    public func clamp(min: Int32, max: Int32) -> Int32 {
        if min > max {
            fatalError("clamp: min must be <= max")
        }
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

    /// Left shift by `count`. The shift count is masked to
    /// `count % bitWidth` (matching hardware semantics), so
    /// `count >= bitWidth` wraps around rather than zeroing the value —
    /// e.g. shifting a `Int32` left by `bitWidth` is a no-op.
    public consuming func shiftLeft(consuming by count: Int64) -> Int32 { Int32(raw: lang.i32_shl(self.raw, lang.cast_i64_i32(count.raw))) }

    /// Right shift by `count`. Arithmetic (sign-extending) for signed types,
    /// logical (zero-filling) for unsigned. The count is masked to
    /// `count % bitWidth`, same as `shiftLeft`.
    public consuming func shiftRight(consuming by count: Int64) -> Int32 { Int32(raw: lang.i32_signed_shr(self.raw, lang.cast_i64_i32(count.raw))) }

    /// Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
    /// MSB re-enter at the LSB.
    public func rotateLeft(by count: Int64) -> Int32 {
        let bits: Int64 = 32;
        let c = count.modulo(by: bits);
        if c == 0 { self }
        else { self.shiftLeft(by: c).bitwiseOr(self.shiftRight(by: bits - c)) }
    }

    /// Rotates bits right by `count`, modulo `bitWidth`. Mirror of
    /// `rotateLeft`.
    public func rotateRight(by count: Int64) -> Int32 {
        let bits: Int64 = 32;
        let c = count.modulo(by: bits);
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
    public mutating func remainderAssign(other: Int32) { self = self.remainder(other) }
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
            result.append(unsafe ptr.offset(by: i).read());
            i = i + 1
        }
        result
    }

    /// Splits this integer into 4 bytes in big-endian order (most
    /// significant byte first — i.e. network byte order).
    public func toBytesBigEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 4);
        let value = UInt64(truncating: self);
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < 4 {
            let shift = (4 - 1 - i) * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(truncating: byteVal));
            i = i + 1
        }
        result
    }

    /// Splits this integer into 4 bytes in little-endian order (least
    /// significant byte first).
    public func toBytesLittleEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 4);
        let value = UInt64(truncating: self);
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < 4 {
            let shift = i * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(truncating: byteVal));
            i = i + 1
        }
        result
    }

    /// @name From Bytes
    /// Reassembles a `Int32` from 4 bytes in native byte order.
    /// Returns `null` if the input is not exactly 4 bytes long.
    public init[S](fromBytes fromBytes: &S)? where S: Sliceable[UInt8], S: not Static, S: not Copyable, S: not Sized {
        let bytes = fromBytes.asUnsafeSlice();
        if bytes.count != 4 {
            return null
        }
        var value = Int32.zero;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 4 {
            unsafe ptr.offset(by: i).write(bytes(unchecked: i));
            i = i + 1
        }
        self.raw = value.raw;
    }

    /// @name From Bytes Big Endian
    /// Reassembles a `Int32` from 4 bytes in big-endian order.
    /// Returns `null` if the input is not exactly 4 bytes long.
    public init[S](fromBytesBigEndian fromBytesBigEndian: &S)? where S: Sliceable[UInt8], S: not Static, S: not Copyable, S: not Sized {
        let bytes = fromBytesBigEndian.asUnsafeSlice();
        if bytes.count != 4 {
            return null
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 4 {
            let byteVal = UInt64(from: unsafe bytes(unchecked: i));
            result = (result << 8) | byteVal;
            i = i + 1
        }
        self.raw = Int32(truncating: result).raw;
    }

    /// @name From Bytes Little Endian
    /// Reassembles a `Int32` from 4 bytes in little-endian order.
    /// Returns `null` if the input is not exactly 4 bytes long.
    public init[S](fromBytesLittleEndian fromBytesLittleEndian: &S)? where S: Sliceable[UInt8], S: not Static, S: not Copyable, S: not Sized {
        let bytes = fromBytesLittleEndian.asUnsafeSlice();
        if bytes.count != 4 {
            return null
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 4 {
            let shift = i * 8;
            let byteVal = UInt64(from: unsafe bytes(unchecked: i));
            result = result | (byteVal << shift);
            i = i + 1
        }
        self.raw = Int32(truncating: result).raw;
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
    public init[S](parsing text: S)? where S: Str, S: not Static {
        // One view for the whole parse; works on String and StringSlice alike.
        let bytes = text.bytes;
        let len = bytes.count;
        if len == 0 {
            return null
        }

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = unsafe bytes(unchecked: 0);
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

        // Accumulate the positive magnitude in UInt64 so that minValue's
        // magnitude (maxValue+1) is representable; convert + negate at the end.
        let maxMagnitude: UInt64 = if isNegative {
            UInt64(truncating: Int32.maxValue) + 1
        } else {
            UInt64(truncating: Int32.maxValue)
        };

        var result: UInt64 = 0;

        while index < len {
            let byte: UInt8 = unsafe bytes(unchecked: index);
            let byteVal = Int64(from: byte);

            if byteVal < 48 or byteVal > 57 {
                return null
            }

            let digit = byteVal - 48;
            let digitU: UInt64 = UInt64(truncating: digit);

            if result > (maxMagnitude - digitU) / 10 {
                return null
            }
            result = result * 10 + digitU;

            index = index + 1
        }

        let typedResult = Int32(truncating: result);
        if isNegative {
            self.raw = typedResult.negateWrapping().raw
        } else {
            self.raw = typedResult.raw
        }
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
    public init[S](parsing text: S, radix radix: Int64)? where S: Str, S: not Static {
        // One view for the whole parse; works on String and StringSlice alike.
        let bytes = text.bytes;
        if radix < 2 or radix > 36 {
            return null
        }

        let len = bytes.count;
        if len == 0 {
            return null
        }

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = unsafe bytes(unchecked: 0);
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

        let radixU: UInt64 = UInt64(truncating: radix);
        let maxMagnitude: UInt64 = if isNegative {
            UInt64(truncating: Int32.maxValue) + 1
        } else {
            UInt64(truncating: Int32.maxValue)
        };

        var result: UInt64 = 0;

        while index < len {
            let byte: UInt8 = unsafe bytes(unchecked: index);
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

            let digitU: UInt64 = UInt64(truncating: digit);
            if result > (maxMagnitude - digitU) / radixU {
                return null
            }
            result = result * radixU + digitU;
            index = index + 1
        }

        let typedResult = Int32(truncating: result);
        if isNegative {
            self.raw = typedResult.negateWrapping().raw
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
    /// (42).formatted();                                           // "42"
    /// (255).formatted(.{radix: 16});                     // "ff"
    /// (255).formatted(.{radix: 16, uppercase: true});    // "FF"
    /// (255).formatted(.{radix: 16, alternate: true});    // "0xff"
    /// (42).formatted(.{radix: 2, alternate: true});      // "0b101010"
    /// (42).formatted(.{width: .Some(5), fill: '0'});     // "00042"
    /// (-42).formatted(.{sign: .Always});                 // "-42"
    /// ```
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        var n = self;
        let isNegative = n < 0;

        var radix: Int64 = options.radix;
        if radix < 2 or radix > 36 {
            radix = 10
        }

        // Build digits in reverse order (need random access to reverse)
        var digits = String();
        if n == Int32.zero {
            digits.appendByte(48)
        } else {
        // Convert to unsigned magnitude so minValue formats correctly.
        // negate() overflows on minValue; unsigned subtraction from zero does not.
        let mag: UInt32 = if isNegative {
            UInt32.zero.subtractWrapping(UInt32(truncating: n))
        } else {
            UInt32(truncating: n)
        };
        let radixVal: UInt32 = UInt32(truncating: radix);
        var m = mag;
        while m != UInt32.zero {
            let digit: UInt32 = m % radixVal;
            let digitVal: Int64 = Int64(from: digit);
            let charCode: Int64 = if digitVal < 10 {
                digitVal + 48
            } else if options.uppercase {
                digitVal - 10 + 65
            } else {
                digitVal - 10 + 97
            };
            digits.appendByte(UInt8(truncating: charCode));
            m = m / radixVal
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

        var i = digits.bytes.count - 1;
        while i >= 0 {
            result.appendByte(unsafe digits.bytes(unchecked: i));
            i = i - 1
        }

        _writePadded(into: writer, result, options)
    }}

