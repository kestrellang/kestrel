// Bitwise operator protocols
// These protocols define bitwise operations for integer types.

module std.core

/// Raw protocol backing the `&` operator.
///
/// Implemented by every integer width; `Output` is `Self` for the standard
/// integer types but may differ for SIMD or bitset wrappers.
///
/// # Examples
///
/// ```
/// 0b1100 & 0b1010   // 0b1000
/// ```
@builtin(.BitwiseAndOperatorProtocol)
public protocol BitwiseAnd[Rhs = Self] {
    type Output

    /// Returns `self & other`.
    @builtin(.BitwiseAndOperatorMethod)
    func bitwiseAnd(other: Rhs) -> Output
}

/// Raw protocol backing the `|` operator.
@builtin(.BitwiseOrOperatorProtocol)
public protocol BitwiseOr[Rhs = Self] {
    type Output

    /// Returns `self | other`.
    @builtin(.BitwiseOrOperatorMethod)
    func bitwiseOr(other: Rhs) -> Output
}

/// Raw protocol backing the `^` operator.
@builtin(.BitwiseXorOperatorProtocol)
public protocol BitwiseXor[Rhs = Self] {
    type Output

    /// Returns `self ^ other`.
    @builtin(.BitwiseXorOperatorMethod)
    func bitwiseXor(other: Rhs) -> Output
}

/// Raw protocol backing the unary `~` operator.
@builtin(.BitwiseNotOperatorProtocol)
public protocol BitwiseNot {
    type Output

    /// Returns `~self` — every bit flipped.
    @builtin(.BitwiseNotOperatorMethod)
    func bitwiseNot() -> Output
}

/// Raw protocol backing the `<<` operator.
///
/// `Rhs` defaults to the primitive `lang.i64` because protocol type defaults
/// must be resolvable at parse time, before stdlib types like `Int64` are
/// available. Conforming integer types narrow this to a more specific shift
/// count where appropriate.
///
/// # Errors
///
/// Standard integer types panic on out-of-range shift counts (see the
/// `shiftLeft` documentation on the integer types).
@builtin(.ShiftLeftOperatorProtocol)
public protocol LeftShift[Rhs = lang.i64] {
    type Output

    /// Returns `self << count`.
    @builtin(.ShiftLeftOperatorMethod)
    func shiftLeft(by count: Rhs) -> Output
}

/// Raw protocol backing the `>>` operator.
///
/// Behaviour for signed types is arithmetic shift (sign-preserving); unsigned
/// types use logical shift. The `Rhs` default mirrors `LeftShift`.
@builtin(.ShiftRightOperatorProtocol)
public protocol RightShift[Rhs = lang.i64] {
    type Output

    /// Returns `self >> count`.
    @builtin(.ShiftRightOperatorMethod)
    func shiftRight(by count: Rhs) -> Output
}
