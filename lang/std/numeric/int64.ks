// Int64 - 64-bit signed integer
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
    ExpressibleByIntLiteral, Convertible, NarrowsFrom, Defaultable,
    RangeConstructible, ClosedRangeConstructible, Range, ClosedRange,
    RangeFromConstructible, RangeUpToConstructible, RangeThroughConstructible,
    RangeFrom, RangeUpTo, RangeThrough
)
import std.text.(String, StringSlice, StringBuilder, Formatter, Formattable, FormatOptions, _writePadded)
import std.memory.(UnsafeSlice, Pointer)
import std.collections.(Slice)
import std.numeric.(UInt8, Int64, UInt64, Float32, Float64)

/// A 64-bit signed integer.
///
/// Int64 is the 64-bit member of the integer family. The same surface
/// area is provided across all widths; switch widths to trade range for memory
/// or to match an FFI ABI. Arithmetic traps on overflow in unoptimized builds and wraps in
/// optimized ones (`--overflow-checks` overrides). Use the `*Wrapping`,
/// `*Checked`, `*Saturating` or `*Overflowing` variants to pick a behaviour
/// that holds in every build. The type is `FFISafe` and lays out as a single
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
    Convertible[Int64],
    Convertible[Int8],
    Convertible[Int16],
    Convertible[Int32],
    Convertible[UInt8],
    Convertible[UInt16],
    Convertible[UInt32],
    NarrowsFrom[UInt64],
    NarrowsFrom[Float32],
    NarrowsFrom[Float64]
{
    /// The underlying primitive `lang.i64` value. Exposed for FFI
    /// and intrinsic use; prefer the typed surface for everything else.
    public var raw: lang.i64

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    /// The additive identity, `0`.
    public static var zero: Int64 { 0 }

    /// The multiplicative identity, `1`.
    public static var one: Int64 { 1 }

    /// The smallest representable value.
    /// This is -2^63 (-9_223_372_036_854_775_808).
    /// Note that for signed types `minValue.negate()` overflows back to
    /// itself; use `negateChecked()` if you need to detect that.
    public static var minValue: Int64 { Int64(raw: lang.i64_shl(1, 63)) }

    /// The largest representable value.
    /// This is 2^63 - 1 (9_223_372_036_854_775_807).
    public static var maxValue: Int64 { 9223372036854775807 }

    /// The width in bits (64). Useful for shift bounds and bit-walks.
    public static var bitWidth: Int64 { 64 }

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
    /// Wraps an existing `lang.i64` without conversion. The FFI
    /// counterpart of `raw`, for values coming back from intrinsics or C.
    public init(raw value: lang.i64) {
        self.raw = value
    }

    /// @name Identity
    /// `Int64` from itself, so a `Convertible[T]` bound accepts `Int64`.
    public init(from other: Int64) { self.raw = other.raw }
    /// @name From Integer
    /// Converts from `Int8`. Lossless: every `Int8` fits in `Int64`.
    public init(from other: Int8) { self.raw = lang.cast_i8_i64(other.raw) }
    /// @name From Integer
    /// Converts from `Int16`. Lossless: every `Int16` fits in `Int64`.
    public init(from other: Int16) { self.raw = lang.cast_i16_i64(other.raw) }
    /// @name From Integer
    /// Converts from `Int32`. Lossless: every `Int32` fits in `Int64`.
    public init(from other: Int32) { self.raw = lang.cast_i32_i64(other.raw) }
    /// @name From Integer
    /// Converts from `UInt8`. Lossless: every `UInt8` fits in `Int64`.
    public init(from other: UInt8) { self.raw = lang.cast_u8_i64(other.raw) }
    /// @name From Integer
    /// Converts from `UInt16`. Lossless: every `UInt16` fits in `Int64`.
    public init(from other: UInt16) { self.raw = lang.cast_u16_i64(other.raw) }
    /// @name From Integer
    /// Converts from `UInt32`. Lossless: every `UInt32` fits in `Int64`.
    public init(from other: UInt32) { self.raw = lang.cast_u32_i64(other.raw) }
    /// @name Exactly
    /// Converts from `UInt64` when the value fits in `Int64`, and
    /// is `nil` when it doesn't. `UInt64` doesn't always fit, so there is
    /// no `init(from:)`: use `init(truncating:)` or `init(clamping:)` to say
    /// what an out-of-range value should become.
    public init(exactly other: UInt64)? {
        if other > UInt64(truncating: Int64.maxValue) {
            return null
        }
        self.raw = other.raw
    }
    /// @name Truncating
    /// Converts from `UInt64` keeping only the low bits, reinterpreted as
    /// `Int64`. Never traps.
    public init(truncating other: UInt64) { self.raw = other.raw }
    /// @name Clamping
    /// Converts from `UInt64`, saturating to `Int64.minValue` / `Int64.maxValue`
    /// when the value doesn't fit.
    public init(clamping other: UInt64) {
        if other > UInt64(truncating: Int64.maxValue) {
            self.raw = Int64.maxValue.raw
        } else {
            self.raw = other.raw
        }
    }
    /// @name Bit Pattern
    /// Reinterprets the bits of a same-width `UInt64` as `Int64`. Never traps.
    public init(bitPattern other: UInt64) { self.raw = other.raw }
    /// @name Exactly
    /// Converts from `Float32` when the value is a whole number in
    /// `Int64`'s range, and is `nil` otherwise (a fraction, NaN, an
    /// infinity, or out of range).
    public init(exactly other: Float32)? {
        // Written so a NaN fails the range test too.
        if not (other >= -9223372036854775808.0 and other < 9223372036854775808.0) {
            return null
        }
        if other.trunc() != other {
            return null
        }
        self.raw = lang.cast_f32_i64(other.raw)
    }
    /// @name Truncating
    /// Converts from `Float32`, dropping any fraction toward zero.
    /// Out-of-range values saturate to `Int64.minValue` /
    /// `Int64.maxValue` and NaN becomes 0. Never traps.
    public init(truncating other: Float32) {
        self.raw = lang.cast_f32_i64(other.raw)
    }
    /// @name Clamping
    /// Same as `init(truncating:)`: drops the fraction, saturates out of
    /// range, and NaN becomes 0. Never traps.
    public init(clamping other: Float32) {
        self.raw = lang.cast_f32_i64(other.raw)
    }
    /// @name Exactly
    /// Converts from `Float64` when the value is a whole number in
    /// `Int64`'s range, and is `nil` otherwise (a fraction, NaN, an
    /// infinity, or out of range).
    public init(exactly other: Float64)? {
        // Written so a NaN fails the range test too.
        if not (other >= -9223372036854775808.0 and other < 9223372036854775808.0) {
            return null
        }
        if other.trunc() != other {
            return null
        }
        self.raw = lang.cast_f64_i64(other.raw)
    }
    /// @name Truncating
    /// Converts from `Float64`, dropping any fraction toward zero.
    /// Out-of-range values saturate to `Int64.minValue` /
    /// `Int64.maxValue` and NaN becomes 0. Never traps.
    public init(truncating other: Float64) {
        self.raw = lang.cast_f64_i64(other.raw)
    }
    /// @name Clamping
    /// Same as `init(truncating:)`: drops the fraction, saturates out of
    /// range, and NaN becomes 0. Never traps.
    public init(clamping other: Float64) {
        self.raw = lang.cast_f64_i64(other.raw)
    }

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
        64 - self.countOnes
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
    /// (42).isEqual(to: 42);  // true
    /// 42 == 42;               // true
    /// ```
    public func isEqual(to other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    /// Pattern-matching hook for `Matchable`. Identical to `isEqual`.
    public func matches(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
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
    public func compare(other: Int64) -> Ordering {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i64_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Successor — `self + 1`. Overflows at `maxValue`, like `add`. Used by `for-in` over
    /// integer ranges.
    public func successor() -> Int64 { self.add(Int64.one) }

    /// Predecessor — `self - 1`. Overflows at `minValue`, like `subtract`.
    public func predecessor() -> Int64 { self.subtract(Int64.one) }

    /// Number of `successor()` steps from `self` to `other` — `other - self`
    /// widened to `Int64` (negative when `other < self`). `O(1)`; lets closed
    /// ranges iterate with a counter instead of a "finished" flag.
    public func distance(to other: Int64) -> Int64 {
        other.subtract(self)
    }

    /// Builds a half-open range `self..<end`. Sugar for the `..<` operator.
    public func exclusiveRange(to end: Int64) -> Range[Int64] {
        Range[Int64](self, end)
    }

    /// Builds a closed range `self..=end`. Sugar for the `..=` operator.
    public func inclusiveRange(to end: Int64) -> ClosedRange[Int64] {
        ClosedRange[Int64](self, end)
    }

    /// Builds a partial range `self..` (from self, no upper bound).
    public func rangeFrom() -> RangeFrom[Int64] {
        RangeFrom[Int64](self)
    }

    /// Builds a partial range `..<self` (up to self, exclusive).
    public func rangeUpTo() -> RangeUpTo[Int64] {
        RangeUpTo[Int64](self)
    }

    /// Builds a partial range `..=self` (through self, inclusive).
    public func rangeThrough() -> RangeThrough[Int64] {
        RangeThrough[Int64](self)
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

    type Addable.Output = Int64
    type Subtractable.Output = Int64
    type Multipliable.Output = Int64
    type Divisible.Output = Int64
    type Remainder.Output = Int64
    type Negatable.Output = Int64
    type BitwiseAnd.Output = Int64
    type BitwiseOr.Output = Int64
    type BitwiseXor.Output = Int64
    type BitwiseNot.Output = Int64
    type LeftShift.Output = Int64
    type RightShift.Output = Int64
    type RangeConstructible.Output = Range[Int64]
    type ClosedRangeConstructible.Output = ClosedRange[Int64]
    type RangeFromConstructible.Output = RangeFrom[Int64]
    type RangeUpToConstructible.Output = RangeUpTo[Int64]
    type RangeThroughConstructible.Output = RangeThrough[Int64]

    // ========================================================================
    // ARITHMETIC (Overflow-checked)
    // ========================================================================

    /// `self + other`. Traps on overflow when overflow checks are on; use `addWrapping`, `addChecked`,
    /// `addSaturating` or `addOverflowing` when overflow is expected.
    ///
    /// # Errors
    ///
    /// Panics when the result doesn't fit in `Int64` and overflow checks are
    /// on (the default at `-O0`). With checks off it wraps.
    public consuming func add(consuming other: Int64) -> Int64 {
        if Bool(boolLiteral: lang.i1_and(lang.overflow_checks(), lang.i64_signed_add_overflows(self.raw, other.raw))) {
            fatalError("arithmetic overflow: Int64 +")
        }
        Int64(raw: lang.i64_add(self.raw, other.raw))
    }

    /// `self + other`, wrapping around on overflow (two's complement).
    /// For hashes, checksums and RNGs where wrap-around is the intent.
    public func addWrapping(other: Int64) -> Int64 { Int64(raw: lang.i64_add(self.raw, other.raw)) }

    /// `self + other` plus whether it overflowed. The value is the
    /// wrapped result, as from `addWrapping`.
    public func addOverflowing(other: Int64) -> (Int64, Bool) {
        (Int64(raw: lang.i64_add(self.raw, other.raw)), Bool(boolLiteral: lang.i64_signed_add_overflows(self.raw, other.raw)))
    }

    /// `self - other`. Traps on overflow when overflow checks are on; use `subtractWrapping`, `subtractChecked`,
    /// `subtractSaturating` or `subtractOverflowing` when overflow is expected.
    ///
    /// # Errors
    ///
    /// Panics when the result doesn't fit in `Int64` and overflow checks are
    /// on (the default at `-O0`). With checks off it wraps.
    public consuming func subtract(consuming other: Int64) -> Int64 {
        if Bool(boolLiteral: lang.i1_and(lang.overflow_checks(), lang.i64_signed_sub_overflows(self.raw, other.raw))) {
            fatalError("arithmetic overflow: Int64 -")
        }
        Int64(raw: lang.i64_sub(self.raw, other.raw))
    }

    /// `self - other`, wrapping around on overflow (two's complement).
    /// For hashes, checksums and RNGs where wrap-around is the intent.
    public func subtractWrapping(other: Int64) -> Int64 { Int64(raw: lang.i64_sub(self.raw, other.raw)) }

    /// `self - other` plus whether it overflowed. The value is the
    /// wrapped result, as from `subtractWrapping`.
    public func subtractOverflowing(other: Int64) -> (Int64, Bool) {
        (Int64(raw: lang.i64_sub(self.raw, other.raw)), Bool(boolLiteral: lang.i64_signed_sub_overflows(self.raw, other.raw)))
    }

    /// `self * other`. Traps on overflow when overflow checks are on; use `multiplyWrapping`, `multiplyChecked`,
    /// `multiplySaturating` or `multiplyOverflowing` when overflow is expected.
    ///
    /// # Errors
    ///
    /// Panics when the result doesn't fit in `Int64` and overflow checks are
    /// on (the default at `-O0`). With checks off it wraps.
    public consuming func multiply(consuming other: Int64) -> Int64 {
        if Bool(boolLiteral: lang.i1_and(lang.overflow_checks(), lang.i64_signed_mul_overflows(self.raw, other.raw))) {
            fatalError("arithmetic overflow: Int64 *")
        }
        Int64(raw: lang.i64_mul(self.raw, other.raw))
    }

    /// `self * other`, wrapping around on overflow (two's complement).
    /// For hashes, checksums and RNGs where wrap-around is the intent.
    public func multiplyWrapping(other: Int64) -> Int64 { Int64(raw: lang.i64_mul(self.raw, other.raw)) }

    /// `self * other` plus whether it overflowed. The value is the
    /// wrapped result, as from `multiplyWrapping`.
    public func multiplyOverflowing(other: Int64) -> (Int64, Bool) {
        (Int64(raw: lang.i64_mul(self.raw, other.raw)), Bool(boolLiteral: lang.i64_signed_mul_overflows(self.raw, other.raw)))
    }

    /// The full double-width product `self * other` as `(low, high)`
    /// halves: `low` is the wrapped product (as from `multiplyWrapping`),
    /// `high` the bits above it. Compiles to one widening multiply. For
    /// hashes, RNGs and bignum arithmetic.
    public func multiplyFull(other: Int64) -> (Int64, Int64) {
        (Int64(raw: lang.i64_mul(self.raw, other.raw)), Int64(raw: lang.i64_signed_mul_high(self.raw, other.raw)))
    }

    /// Truncating integer division (`self / other`). For signed types,
    /// `minValue / -1` traps when overflow checks are on (and wraps
    /// when off); use `divideWrapping` or `divideChecked`.
    ///
    /// # Errors
    ///
    /// Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
    /// process aborts before producing a result).
    public consuming func divide(consuming other: Int64) -> Int64 {
        if Bool(boolLiteral: lang.overflow_checks()) and self == Int64.minValue and other == -1 {
            fatalError("arithmetic overflow: Int64.minValue / -1")
        }
        Int64(raw: lang.i64_signed_div(self.raw, other.raw))
    }

    /// `self / other`, with `minValue / -1` wrapping to `minValue`.
    /// Still traps on division by zero.
    public func divideWrapping(other: Int64) -> Int64 { Int64(raw: lang.i64_signed_div(self.raw, other.raw)) }

    /// `self % other` — truncated remainder; the result has the sign of
    /// `self` for signed types.
    ///
    /// # Errors
    ///
    /// Traps on division by zero, like `divide`.
    public consuming func remainder(consuming other: Int64) -> Int64 { Int64(raw: lang.i64_signed_rem(self.raw, other.raw)) }

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
    public func modulo(by other: Int64) -> Int64 {
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
    public consuming func divideUnchecked(consuming other: Int64) -> Int64 { Int64(raw: lang.i64_signed_div_unchecked(self.raw, other.raw)) }

    /// `self % other` without the divide-by-zero and `minValue % -1` guards.
    /// Same safety contract as `divideUnchecked`.
    ///
    /// # Safety
    ///
    /// UB when `other == 0`, or signed `minValue % -1`.
    public consuming func remainderUnchecked(consuming other: Int64) -> Int64 { Int64(raw: lang.i64_signed_rem_unchecked(self.raw, other.raw)) }

    /// Two's-complement negation. Traps at the minimum value, which has
    /// no positive counterpart, when overflow checks are on (wraps when off); use `negateWrapping`, `negateChecked` or
    /// `negateSaturating` there.
    public consuming func negate() -> Int64 {
        if Bool(boolLiteral: lang.overflow_checks()) and self == Int64.minValue {
            fatalError("arithmetic overflow: -Int64.minValue")
        }
        Int64(raw: lang.i64_neg(self.raw))
    }

    /// `-self`, with `-minValue` wrapping back to `minValue`.
    public func negateWrapping() -> Int64 { Int64(raw: lang.i64_neg(self.raw)) }
    /// Absolute value. Traps at the minimum value when overflow checks are on; use `absWrapping`,
    /// `absChecked` or `absSaturating` there.
    public func abs() -> Int64 { if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { self.negate() } else { self } }

    /// `|self|`, with `minValue.absWrapping()` wrapping to `minValue`.
    public func absWrapping() -> Int64 { if Bool(boolLiteral: lang.i64_signed_lt(self.raw, 0)) { self.negateWrapping() } else { self } }

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    /// `self + other`, or `None` on overflow.
    public func addChecked(other: Int64) -> Int64? {
        if Bool(boolLiteral: lang.i64_signed_add_overflows(self.raw, other.raw)) { return .None };
        .Some(self.addWrapping(other))
    }

    /// `self - other`, or `None` on overflow.
    public func subtractChecked(other: Int64) -> Int64? {
        if Bool(boolLiteral: lang.i64_signed_sub_overflows(self.raw, other.raw)) { return .None };
        .Some(self.subtractWrapping(other))
    }

    /// `self * other`, or `None` on overflow.
    public func multiplyChecked(other: Int64) -> Int64? {
        if Bool(boolLiteral: lang.i64_signed_mul_overflows(self.raw, other.raw)) { return .None };
        .Some(self.multiplyWrapping(other))
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
        .Some(self.divideWrapping(other))
    }

    /// Negation that returns `None` for `minValue` (whose negation overflows).
    public func negateChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.negateWrapping())
    }

    /// Absolute value that returns `None` for `minValue` (whose absolute
    /// value overflows).
    public func absChecked() -> Int64? {
        if self == Int64.minValue {
            return .None
        };
        .Some(self.absWrapping())
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
    public func pow(exponent: Int64) -> Int64 {
        if Bool(boolLiteral: lang.i1_not(lang.overflow_checks())) {
            return self.powWrapping(exponent)
        }
        match self.powChecked(exponent) {
            .Some(value) => value,
            .None => fatalError("arithmetic overflow: Int64.pow")
        }
    }

    /// `self` raised to `exponent`, or `None` on overflow or a negative
    /// exponent.
    public func powChecked(exponent: Int64) -> Int64? {
        if exponent < 0 {
            return .None
        }
        var result: Int64 = 1;
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
    public func powWrapping(exponent: Int64) -> Int64 {
        if exponent < 0 {
            fatalError("pow: negative exponent")
        }
        var result: Int64 = 1;
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
    public func powSaturating(exponent: Int64) -> Int64 {
        if exponent < 0 {
            fatalError("pow: negative exponent")
        }
        match self.powChecked(exponent) {
            .Some(value) => value,
            .None => if self < 0 and exponent % 2 == 1 { Int64.minValue } else { Int64.maxValue }
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
    public func gcd(other: Int64) -> Int64 {
        var a = self.absWrapping();
        var b = other.absWrapping();
        // `minValue.abs()` wraps to `minValue`, so a remainder can come out
        // negative: keep each remainder's magnitude so the result is
        // non-negative (only `gcd(minValue, 0)` = 2^(n-1) is unrepresentable).
        while b != Int64.zero {
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
    public func clamp(min: Int64, max: Int64) -> Int64 {
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
    public consuming func bitwiseAnd(consuming other: Int64) -> Int64 { Int64(raw: lang.i64_and(self.raw, other.raw)) }

    /// Bitwise OR. `0b1010 | 0b1100 == 0b1110`.
    public consuming func bitwiseOr(consuming other: Int64) -> Int64 { Int64(raw: lang.i64_or(self.raw, other.raw)) }

    /// Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.
    public consuming func bitwiseXor(consuming other: Int64) -> Int64 { Int64(raw: lang.i64_xor(self.raw, other.raw)) }

    /// Bitwise NOT — flips all bits. For signed types this is `-self - 1`.
    public consuming func bitwiseNot() -> Int64 { Int64(raw: lang.i64_not(self.raw)) }

    /// Left shift by `count`. The shift count is masked to
    /// `count % bitWidth` (matching hardware semantics), so
    /// `count >= bitWidth` wraps around rather than zeroing the value —
    /// e.g. shifting a `Int64` left by `bitWidth` is a no-op.
    public consuming func shiftLeft(consuming by count: Int64) -> Int64 { Int64(raw: lang.i64_shl(self.raw, count.raw)) }

    /// Right shift by `count`. Arithmetic (sign-extending) for signed types,
    /// logical (zero-filling) for unsigned. The count is masked to
    /// `count % bitWidth`, same as `shiftLeft`.
    public consuming func shiftRight(consuming by count: Int64) -> Int64 { Int64(raw: lang.i64_signed_shr(self.raw, count.raw)) }

    /// Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
    /// MSB re-enter at the LSB.
    public func rotateLeft(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count.modulo(by: bits);
        if c == 0 { self }
        else { self.shiftLeft(by: c).bitwiseOr(self.shiftRight(by: bits - c)) }
    }

    /// Rotates bits right by `count`, modulo `bitWidth`. Mirror of
    /// `rotateLeft`.
    public func rotateRight(by count: Int64) -> Int64 {
        let bits: Int64 = 64;
        let c = count.modulo(by: bits);
        if c == 0 { self }
        else { self.shiftRight(by: c).bitwiseOr(self.shiftLeft(by: bits - c)) }
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
    public mutating func remainderAssign(other: Int64) { self = self.remainder(other) }
    /// `self &= other`
    public mutating func bitwiseAndAssign(other: Int64) { self = self.bitwiseAnd(other) }
    /// `self |= other`
    public mutating func bitwiseOrAssign(other: Int64) { self = self.bitwiseOr(other) }
    /// `self ^= other`
    public mutating func bitwiseXorAssign(other: Int64) { self = self.bitwiseXor(other) }
    /// `self <<= count`
    public mutating func shiftLeftAssign(by count: Int64) { self = self.shiftLeft(by: count) }
    /// `self >>= count`
    public mutating func shiftRightAssign(by count: Int64) { self = self.shiftRight(by: count) }

    // ========================================================================
    // BYTE CONVERSION
    // ========================================================================

    /// Splits this integer into 8 bytes in *native* (host) byte order.
    /// Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
    /// a fixed wire format.
    ///
    /// # Examples
    ///
    /// ```
    /// let bytes = Int64.maxValue.toBytes();   // 8 bytes, host order
    /// ```
    public func toBytes() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 8);
        let value = self;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 8 {
            result.append(unsafe ptr.offset(by: i).read());
            i = i + 1
        }
        result
    }

    /// Splits this integer into 8 bytes in big-endian order (most
    /// significant byte first — i.e. network byte order).
    public func toBytesBigEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 8);
        let value = UInt64(truncating: self);
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < 8 {
            let shift = (8 - 1 - i) * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(truncating: byteVal));
            i = i + 1
        }
        result
    }

    /// Splits this integer into 8 bytes in little-endian order (least
    /// significant byte first).
    public func toBytesLittleEndian() -> std.collections.Array[UInt8] {
        var result = std.collections.Array[UInt8](capacity: 8);
        let value = UInt64(truncating: self);
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < 8 {
            let shift = i * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(truncating: byteVal));
            i = i + 1
        }
        result
    }

    /// @name From Bytes
    /// Reassembles a `Int64` from 8 bytes in native byte order.
    /// Returns `null` if the input is not exactly 8 bytes long.
    public init(fromBytes fromBytes: &Slice[UInt8])? {
        let bytes = fromBytes.asUnsafeSlice();
        if bytes.count != 8 {
            return null
        }
        var value = Int64.zero;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 8 {
            unsafe ptr.offset(by: i).write(bytes(unchecked: i));
            i = i + 1
        }
        self.raw = value.raw;
    }

    /// @name From Bytes Big Endian
    /// Reassembles a `Int64` from 8 bytes in big-endian order.
    /// Returns `null` if the input is not exactly 8 bytes long.
    public init(fromBytesBigEndian fromBytesBigEndian: &Slice[UInt8])? {
        let bytes = fromBytesBigEndian.asUnsafeSlice();
        if bytes.count != 8 {
            return null
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 8 {
            let byteVal = UInt64(from: unsafe bytes(unchecked: i));
            result = (result << 8) | byteVal;
            i = i + 1
        }
        self.raw = Int64(truncating: result).raw;
    }

    /// @name From Bytes Little Endian
    /// Reassembles a `Int64` from 8 bytes in little-endian order.
    /// Returns `null` if the input is not exactly 8 bytes long.
    public init(fromBytesLittleEndian fromBytesLittleEndian: &Slice[UInt8])? {
        let bytes = fromBytesLittleEndian.asUnsafeSlice();
        if bytes.count != 8 {
            return null
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 8 {
            let shift = i * 8;
            let byteVal = UInt64(from: unsafe bytes(unchecked: i));
            result = result | (byteVal << shift);
            i = i + 1
        }
        self.raw = Int64(truncating: result).raw;
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    /// @name Parsing
    /// Parses a base-10 integer literal, optionally prefixed with `+` or `-`.
    /// Returns `null` for an empty string, a non-digit character,
    /// or a value that does not fit in `Int64`.
    ///
    /// # Examples
    ///
    /// ```
    /// Int64(parsing: "42");    // Some(42)
    /// Int64(parsing: "-7");    // Some(-7)
    /// Int64(parsing: "abc");   // None
    /// ```
    public init(parsing text: &StringSlice)? {
        // One view for the whole parse; a String argument lends its slice.
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
            UInt64(truncating: Int64.maxValue) + 1
        } else {
            UInt64(truncating: Int64.maxValue)
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

        let typedResult = Int64(truncating: result);
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
    /// Int64(parsing: "ff", radix: 16);     // Some(255 if it fits, else None)
    /// Int64(parsing: "101010", radix: 2);  // Some(42)
    /// Int64(parsing: "z", radix: 36);      // Some(35)
    /// ```
    public init(parsing text: &StringSlice, radix radix: Int64)? {
        // One view for the whole parse; a String argument lends its slice.
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
            UInt64(truncating: Int64.maxValue) + 1
        } else {
            UInt64(truncating: Int64.maxValue)
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

        let typedResult = Int64(truncating: result);
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
        if n == Int64.zero {
            digits.appendByte(48)
        } else {
        // Convert to unsigned magnitude so minValue formats correctly.
        // negate() overflows on minValue; unsigned subtraction from zero does not.
        let mag: UInt64 = if isNegative {
            UInt64.zero.subtractWrapping(UInt64(truncating: n))
        } else {
            UInt64(truncating: n)
        };
        let radixVal: UInt64 = UInt64(truncating: radix);
        var m = mag;
        while m != UInt64.zero {
            let digit: UInt64 = m % radixVal;
            let digitVal: Int64 = Int64(truncating: digit);
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

/// Platform-sized signed integer — currently always `Int64`.
public type Int = Int64
