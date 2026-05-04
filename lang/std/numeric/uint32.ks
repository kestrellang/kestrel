// UInt32 - 32-bit unsigned integer
// Generated from integer.ks.template (docs synced from .ks.interface) - DO NOT EDIT

module std.numeric

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
import std.numeric.(UInt8, Int64, UInt64)

/// A 32-bit unsigned integer.
///
/// UInt32 is the 32-bit member of the integer family. The same surface
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
    LeftShiftAssign[Int64],
    RightShiftAssign[Int64],
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
    Convertible[UInt64]
{
    /// The underlying primitive `lang.i32` value. Exposed for FFI
    /// and intrinsic use; prefer the typed surface for everything else.
    public var raw: lang.i32

    // ========================================================================
    // CONSTANTS
    // ========================================================================

    /// The additive identity, `0`.
    public static var zero: UInt32 { 0 }

    /// The multiplicative identity, `1`.
    public static var one: UInt32 { 1 }

    /// The smallest representable value.
    /// This is always 0 for unsigned types.
    /// Note that for signed types `minValue.negate()` overflows back to
    /// itself; use `negateChecked()` if you need to detect that.
    public static var minValue: UInt32 { UInt32(intLiteral: 0) }

    /// The largest representable value.
    /// This is 2^32 - 1 (4_294_967_295).
    public static var maxValue: UInt32 { 4294967295 }

    /// The width in bits (32). Useful for shift bounds and bit-walks.
    public static var bitWidth: Int64 { 32 }

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// @name Int Literal
    /// Compiler-emitted bridge that turns an integer literal into a UInt32.
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
    /// Converts from `Int32`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: Int32) { self.raw = other.raw }
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
    /// Converts from `UInt64`. Narrowing conversions truncate the high
    /// bits; signed→unsigned reinterprets the bit pattern.
    public init(from other: UInt64) { self.raw = lang.cast_u64_i32(other.raw) }

    // ========================================================================
    // SIGN INSPECTION (Properties)
    // ========================================================================

    /// Sign as a `UInt32`: `0` for zero, `1` otherwise (unsigned types
    /// have no negative values).
    public var sign: UInt32 { get {
        if Bool(boolLiteral: lang.i32_eq(self.raw, 0)) { UInt32.zero }
        else { UInt32.one }
    }}

    /// True when `self > 0`.
    public var isPositive: Bool { get {
        Bool(boolLiteral: lang.i32_unsigned_gt(self.raw, 0))
    }}

    /// Always `false` — unsigned types cannot be negative.
    public var isNegative: Bool { get {
        // Unsigned types are never negative
        false
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
        if Bool(boolLiteral: lang.i32_eq(self.raw, 0)) { false }
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
    public var byteSwapped: UInt32 { get {
        UInt32(raw: lang.i32_bswap(self.raw))
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
    public func isEqual(to other: UInt32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    /// Pattern-matching hook for `Matchable`. Identical to `isEqual`.
    public func matches(other: UInt32) -> Bool {
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
    public func compare(other: UInt32) -> Ordering {
        if Bool(boolLiteral: lang.i32_unsigned_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i32_unsigned_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // ========================================================================
    // STEPPING
    // ========================================================================

    /// Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
    /// integer ranges.
    public func successor() -> UInt32 { self.add(UInt32.one) }

    /// Predecessor — `self - 1`. Wraps at `minValue`.
    public func predecessor() -> UInt32 { self.subtract(UInt32.one) }

    /// Builds a half-open range `self..<end`. Sugar for the `..<` operator.
    public func exclusiveRange(to end: UInt32) -> Range[UInt32] {
        Range[UInt32](self, end)
    }

    /// Builds a closed range `self..=end`. Sugar for the `..=` operator.
    public func inclusiveRange(to end: UInt32) -> ClosedRange[UInt32] {
        ClosedRange[UInt32](self, end)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
    /// only within a single process — do not persist hashes across builds.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Layout.of[UInt32]().size))
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

    /// `self + other`, wrapping on overflow. Use `addChecked` to detect or
    /// `addSaturating` to clamp.
    public func add(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_add(self.raw, other.raw)) }

    /// `self - other`, wrapping on overflow.
    public func subtract(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_sub(self.raw, other.raw)) }

    /// `self * other`, wrapping on overflow.
    public func multiply(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_mul(self.raw, other.raw)) }

    /// Truncating integer division (`self / other`). For signed types,
    /// `minValue / -1` wraps; use `divideChecked` to detect.
    ///
    /// # Errors
    ///
    /// Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
    /// process aborts before producing a result).
    public func divide(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_div(self.raw, other.raw)) }

    /// `self % other` — truncated remainder; the result has the sign of
    /// `self` for signed types.
    ///
    /// # Errors
    ///
    /// Traps on division by zero, like `divide`.
    public func modulo(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_unsigned_rem(self.raw, other.raw)) }

    
    

    // ========================================================================
    // ARITHMETIC (Checked - Returns Optional)
    // ========================================================================

    // TODO: requires overflow-detecting intrinsics for proper implementation
    /// Wrapping addition that returns `None` on overflow. For unsigned types
    /// overflow is detected via `result < self`.
    public func addChecked(other: UInt32) -> UInt32? {
        let result = self.add(other);
        // For unsigned, overflow if result < either operand
        if result < self {
            return .None
        };
        .Some(result)
    }

    /// Subtraction that returns `None` on underflow (`other > self`).
    public func subtractChecked(other: UInt32) -> UInt32? {
        // For unsigned, underflow if other > self
        if other > self {
            return .None
        };
        .Some(self.subtract(other))
    }

    /// Wrapping multiplication that returns `None` on overflow. Implemented
    /// by multiplying then dividing back.
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

    /// Division that returns `None` for divide-by-zero.
    public func divideChecked(other: UInt32) -> UInt32? {
        if other == UInt32.zero {
            return .None
        };
        .Some(self.divide(other))
    }


    // ========================================================================
    // ARITHMETIC (Saturating - Clamps to Bounds)
    // ========================================================================

    /// Addition that clamps to `maxValue` on overflow.
    public func addSaturating(other: UInt32) -> UInt32 {
        let checked = self.addChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt32.maxValue
        }
    }

    /// Subtraction that clamps to `0` on underflow (unsigned types cannot
    /// represent negative results).
    public func subtractSaturating(other: UInt32) -> UInt32 {
        let checked = self.subtractChecked(other);
        match checked {
            .Some(result) => result,
            .None => UInt32.zero
        }
    }

    /// Multiplication that clamps to `maxValue` on overflow.
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
    public func clamp(min: UInt32, max: UInt32) -> UInt32 {
        if self < min { min }
        else if self > max { max }
        else { self }
    }

    // ========================================================================
    // BITWISE OPERATIONS
    // ========================================================================

    /// Bitwise AND. `0b1010 & 0b1100 == 0b1000`.
    public func bitwiseAnd(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_and(self.raw, other.raw)) }

    /// Bitwise OR. `0b1010 | 0b1100 == 0b1110`.
    public func bitwiseOr(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_or(self.raw, other.raw)) }

    /// Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.
    public func bitwiseXor(other: UInt32) -> UInt32 { UInt32(raw: lang.i32_xor(self.raw, other.raw)) }

    /// Bitwise NOT — flips all bits. For signed types this is `-self - 1`.
    public func bitwiseNot() -> UInt32 { UInt32(raw: lang.i32_not(self.raw)) }

    /// Left shift by `count`. Behavior is undefined when `count >= bitWidth`
    /// — pre-mask the count if you can't guarantee the bound.
    public func shiftLeft(by count: Int64) -> UInt32 { UInt32(raw: lang.i32_shl(self.raw, lang.cast_i64_i32(count.raw))) }

    /// Right shift by `count`. Arithmetic (sign-extending) for signed types,
    /// logical (zero-filling) for unsigned. Same `count` precondition as
    /// `shiftLeft`.
    public func shiftRight(by count: Int64) -> UInt32 { UInt32(raw: lang.i32_unsigned_shr(self.raw, lang.cast_i64_i32(count.raw))) }

    /// Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
    /// MSB re-enter at the LSB.
    public func rotateLeft(by count: Int64) -> UInt32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftLeft(by: c).bitwiseOr(self.shiftRight(by: bits - c)) }
    }

    /// Rotates bits right by `count`, modulo `bitWidth`. Mirror of
    /// `rotateLeft`.
    public func rotateRight(by count: Int64) -> UInt32 {
        let bits: Int64 = 32;
        let c = count % bits;
        if c == 0 { self }
        else { self.shiftRight(by: c).bitwiseOr(self.shiftLeft(by: bits - c)) }
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
    /// let bytes = UInt32.maxValue.toBytes();   // 4 bytes, host order
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

    /// Reassembles a `UInt32` from 4 bytes in native (host) byte
    /// order. Returns `None` if the input is not exactly 4 bytes long.
    public static func fromBytes(bytes: std.collections.Array[UInt8]) -> UInt32? {
        if bytes.count != 4 {
            return .None
        }
        var value = UInt32.zero;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < 4 {
            ptr.offset(by: i).write(bytes(unchecked: i));
            i = i + 1
        }
        .Some(value)
    }

    /// Reassembles a `UInt32` from 4 bytes in big-endian order.
    /// Returns `None` if the input is not exactly 4 bytes long.
    public static func fromBytesBigEndian(bytes: std.collections.Array[UInt8]) -> UInt32? {
        if bytes.count != 4 {
            return .None
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 4 {
            let byteVal = UInt64(from: bytes(unchecked: i));
            result = (result << 8) | byteVal;
            i = i + 1
        }
        .Some(UInt32(from: result))
    }

    /// Reassembles a `UInt32` from 4 bytes in little-endian order.
    /// Returns `None` if the input is not exactly 4 bytes long.
    public static func fromBytesLittleEndian(bytes: std.collections.Array[UInt8]) -> UInt32? {
        if bytes.count != 4 {
            return .None
        }
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < 4 {
            let shift = i * 8;
            let byteVal = UInt64(from: bytes(unchecked: i));
            result = result | (byteVal << shift);
            i = i + 1
        }
        .Some(UInt32(from: result))
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    /// Parses a base-10 unsigned integer literal, optionally prefixed
    /// with `+`. A leading `-` is rejected. Returns `None` for an empty
    /// string, a non-digit character, or a value that does not fit in
    /// `UInt32`.
    ///
    /// # Examples
    ///
    /// ```
    /// UInt32.parse("42");   // Some(42)
    /// UInt32.parse("-1");   // None  (no sign for unsigned)
    /// UInt32.parse("");     // None
    /// ```
    public static func parse(string: String) -> UInt32? {
        let len = string.byteCount;
        if len == 0 {
            return .None
        }

        var index: Int64 = 0;

        // Check for optional + sign
        let firstByte: UInt8 = string.bytes(unchecked: 0);
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
            let byte: UInt8 = string.bytes(unchecked: index);
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
    /// Parses an unsigned integer in `radix` (base 2–36 inclusive). Letters
    /// a–z are case-insensitive and represent digit values 10–35. A
    /// leading `+` is allowed but a leading `-` is rejected. Returns
    /// `None` for an out-of-range radix, an empty string, an
    /// unrecognised digit, or a value that overflows `UInt32`.
    ///
    /// # Examples
    ///
    /// ```
    /// UInt32.parse("ff", 16);     // Some(255 if it fits, else None)
    /// UInt32.parse("101010", 2);  // Some(42)
    /// ```
    public static func parse(string: String, radix: Int64) -> UInt32? {
        if radix < 2 or radix > 36 {
            return .None
        }

        let len = string.byteCount;
        if len == 0 {
            return .None
        }

        var index: Int64 = 0;

        // Optional `+`; reject leading `-` outright.
        let firstByte: UInt8 = string.bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 43 {
            index = 1
        } else if firstByteVal == 45 {
            return .None
        }

        // Must have at least one digit
        if index >= len {
            return .None
        }

        let radixU: UInt64 = UInt64(from: radix);
        let maxVal: UInt64 = UInt64(from: UInt32.maxValue);

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
                return .None
            };

            if digit >= radix {
                return .None
            }

            let digitU: UInt64 = UInt64(from: digit);
            if result > (maxVal - digitU) / radixU {
                return .None
            }
            result = result * radixU + digitU;
            index = index + 1
        }

        .Some(UInt32(from: result))
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
    /// (255).format(.{radix: 16});                     // "ff"
    /// (255).format(.{radix: 16, uppercase: true});    // "FF"
    /// (255).format(.{radix: 16, alternate: true});    // "0xff"
    /// (42).format(.{radix: 2, alternate: true});      // "0b101010"
    /// (42).format(.{width: .Some(5), fill: '0'});     // "00042"
    /// (-42).format(.{sign: .Always});                 // "-42"
    /// ```
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
        if n == UInt32.zero {
            digits.appendByte(48)  // '0'
        } else {
            let radixVal: UInt32 = UInt32(from: radix);
            while n != UInt32.zero {
                let digit: UInt32 = n % radixVal;
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
            result.appendByte(digits.bytes(unchecked: i));
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

