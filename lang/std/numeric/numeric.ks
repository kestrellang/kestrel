// Numeric protocols

module std.numeric

import std.core.(
    Bool, Comparable, Hashable, ExpressibleByIntLiteral, ExpressibleByFloatLiteral,
    Addable, Subtractable, Multipliable, Divisible, Remainder, Negatable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift
)
import std.numeric.(Int64)

/// A type whose values can be stepped one position at a time. Underpins
/// `for-in` over integer ranges and any other "next/previous" walk where
/// the step size is implicit (`1` for integers).
///
/// `successor` and `predecessor` should be inverses for every interior
/// value; behaviour at the type's edges (`Int64.maxValue.successor()`,
/// for example) follows the same wrapping rules as `add`/`subtract`.
public protocol Steppable {
    /// The next value in the sequence. For integers this is `self + 1`.
    func successor() -> Self
    /// The previous value in the sequence. For integers this is `self - 1`.
    func predecessor() -> Self
    /// The number of `successor()` steps from `self` to `other` (negative
    /// when `other` precedes `self`). For integers this is `other - self`.
    ///
    /// This is the `O(1)` stride distance — it lets range iterators carry a
    /// remaining-element *counter* instead of a boolean "finished" flag,
    /// which is what keeps `for x in a..=b` unrollable (a counter is an
    /// induction variable the optimizer can reason about; a flag is not).
    ///
    /// The result must fit in `Int64`. For spans wider than `Int64` (only
    /// reachable via near-full-width ranges, which never terminate in
    /// practice) the value wraps, following the same edge rule as
    /// `successor`/`predecessor`.
    func distance(to other: Self) -> Int64
}

/// A number: arithmetic that stays in `Self`, ordering, integer literals,
/// and an exact conversion from `Int64`.
///
/// Every integer and float type conforms. Generic code bound by `Numeric`
/// can add, subtract, multiply and divide values and get `Self` back, so a
/// running total needs no casts. The `where` clause names which `Output`
/// each operator protocol means, since all four declare one.
///
/// # Examples
///
/// ```
/// func mean[T](xs: &Slice[T]) -> T where T: Numeric {
///     var total = T.zero;
///     for x in xs { total = total + x; }
///     total / T(exactly: xs.count)!
/// }
/// ```
public protocol Numeric: Addable, Subtractable, Multipliable, Divisible, Comparable, ExpressibleByIntLiteral
    where Self.Addable.Output = Self,
          Self.Subtractable.Output = Self,
          Self.Multipliable.Output = Self,
          Self.Divisible.Output = Self {
    /// @name Exactly
    /// Converts `value` when `Self` holds it exactly, and is `nil` when it
    /// doesn't fit (or, for a float, would round). Generic code turns a
    /// count into `Self` with it: `T(exactly: xs.count)!`.
    init(exactly value: Int64)?
}

/// A number that can be negative: negation stays in `Self`, and `abs()`
/// gives the magnitude.
///
/// Signed integers and floats conform; unsigned integers don't.
public protocol SignedNumeric: Numeric, Negatable where Self.Negatable.Output = Self {
    /// The magnitude of `self`. For two's-complement integers this can
    /// overflow at `minValue`; the concrete types offer `absChecked()`.
    func abs() -> Self
}

/// A fixed-width integer: remainder, bit operations and shifts that stay in
/// `Self`, stepping, hashing, and the type's range.
public protocol Integer: Numeric, Remainder, Steppable, Hashable,
    BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift
    where Self.Remainder.Output = Self,
          Self.BitwiseAnd.Output = Self,
          Self.BitwiseOr.Output = Self,
          Self.BitwiseXor.Output = Self,
          Self.BitwiseNot.Output = Self,
          Self.LeftShift.Output = Self,
          Self.RightShift.Output = Self {
    /// The smallest representable value.
    static var minValue: Self { get }
    /// The largest representable value.
    static var maxValue: Self { get }
    /// The width in bits.
    static var bitWidth: Int64 { get }
}

/// A signed integer type: `Int8` through `Int64`.
public protocol SignedInteger: Integer, SignedNumeric {}

/// An unsigned integer type: `UInt8` through `UInt64`. The compiler also
/// reads this protocol: integer patterns on a conforming type test and tile
/// its raw bits as unsigned.
@builtin(.UnsignedInteger)
public protocol UnsignedInteger: Integer {}

/// An IEEE 754 binary floating-point type: `Float32` and `Float64`.
///
/// Adds float literals, the special values, and the rounding and root
/// functions every float type has.
public protocol FloatingPoint: SignedNumeric, ExpressibleByFloatLiteral {
    /// Positive infinity.
    static var infinity: Self { get }
    /// A quiet NaN. Never equal to anything, itself included; test with `isNaN`.
    static var nan: Self { get }
    /// The gap between `1` and the next larger value.
    static var epsilon: Self { get }
    /// π.
    static var pi: Self { get }
    /// True if `self` is NaN.
    var isNaN: Bool { get }
    /// True if `self` is positive or negative infinity.
    var isInfinite: Bool { get }
    /// True if `self` is neither infinite nor NaN.
    var isFinite: Bool { get }
    /// The largest integral value not above `self`.
    func floor() -> Self
    /// The smallest integral value not below `self`.
    func ceil() -> Self
    /// The nearest integral value, halfway cases away from zero.
    func round() -> Self
    /// `self` with its fractional part dropped.
    func trunc() -> Self
    /// The square root. NaN for negative inputs.
    func sqrt() -> Self
}
