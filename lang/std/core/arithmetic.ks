// Arithmetic operator protocols
// These protocols define the standard arithmetic operations used by numeric types.

module std.core

/// Raw protocol backing the `+` operator.
///
/// `Output` may differ from `Self` and `Other` — this is what allows mixed-type
/// arithmetic (e.g. `Vector + Scalar -> Vector`) without losing precision.
/// The associated `zero` value gives sums (and `Iterator.sum`) a starting
/// point and is the additive identity by definition.
///
/// # Examples
///
/// ```
/// 2 + 3            // 5
/// Int64.zero       // 0
/// ```
@builtin(.AddOperatorProtocol)
public protocol Addable[Other = Self] {
    type Output

    /// The additive identity — a value `z` such that `x + z == x` for all `x`.
    static var zero: Self { get }

    /// Returns `self + other`.
    @builtin(.AddOperatorMethod)
    func add(other: Other) -> Output
}

/// Raw protocol backing the `-` binary operator.
@builtin(.SubtractOperatorProtocol)
public protocol Subtractable[Other = Self] {
    type Output

    /// Returns `self - other`.
    @builtin(.SubtractOperatorMethod)
    func subtract(other: Other) -> Output
}

/// Raw protocol backing the `*` operator.
///
/// The associated `one` value is the multiplicative identity, used as the
/// starting accumulator for products and powers.
///
/// # Examples
///
/// ```
/// 6 * 7         // 42
/// Int64.one     // 1
/// ```
@builtin(.MultiplyOperatorProtocol)
public protocol Multipliable[Other = Self] {
    type Output

    /// The multiplicative identity — a value `o` such that `x * o == x` for all `x`.
    static var one: Self { get }

    /// Returns `self * other`.
    @builtin(.MultiplyOperatorMethod)
    func multiply(other: Other) -> Output
}

/// Raw protocol backing the `/` operator.
///
/// Division by zero is not modelled at the protocol level; conforming types
/// document their own behavior (integer types panic, floats produce `inf`/`nan`).
@builtin(.DivideOperatorProtocol)
public protocol Divisible[Other = Self] {
    type Output

    /// Returns `self / other`.
    @builtin(.DivideOperatorMethod)
    func divide(other: Other) -> Output
}

/// Raw protocol backing the `%` operator.
///
/// For integers this is the remainder of truncated division, with the sign of
/// the dividend. Use `floorMod` (defined on integer types) when you want
/// Euclidean / floor-style remainder semantics.
@builtin(.ModuloOperatorProtocol)
public protocol Modulo[Other = Self] {
    type Output

    /// Returns `self % other`.
    @builtin(.ModuloOperatorMethod)
    func modulo(other: Other) -> Output
}

/// Raw protocol backing the unary `-` operator.
///
/// On signed two's-complement integers, negating the minimum value overflows
/// (e.g. `-Int8.minValue == Int8.minValue`); the operator wraps. Use
/// `checkedNegate` if overflow needs to surface.
@builtin(.NegateOperatorProtocol)
public protocol Negatable {
    type Output

    /// Returns `-self`.
    @builtin(.NegateOperatorMethod)
    func negate() -> Output
}
