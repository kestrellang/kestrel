// Bitwise operator protocols
// These protocols define bitwise operations for integer types.

module std.core

/// Protocol for types that support bitwise AND (&).
@builtin(.BitwiseAndOperatorProtocol)
public protocol BitwiseAnd[Rhs = Self] {
    type Output

    /// Performs bitwise AND of this value with another.
    @builtin(.BitwiseAndOperatorMethod)
    func bitwiseAnd(other: Rhs) -> Output
}

/// Protocol for types that support bitwise OR (|).
@builtin(.BitwiseOrOperatorProtocol)
public protocol BitwiseOr[Rhs = Self] {
    type Output

    /// Performs bitwise OR of this value with another.
    @builtin(.BitwiseOrOperatorMethod)
    func bitwiseOr(other: Rhs) -> Output
}

/// Protocol for types that support bitwise XOR (^).
@builtin(.BitwiseXorOperatorProtocol)
public protocol BitwiseXor[Rhs = Self] {
    type Output

    /// Performs bitwise XOR of this value with another.
    @builtin(.BitwiseXorOperatorMethod)
    func bitwiseXor(other: Rhs) -> Output
}

/// Protocol for types that support bitwise NOT (~).
@builtin(.BitwiseNotOperatorProtocol)
public protocol BitwiseNot {
    type Output

    /// Returns the bitwise complement of this value.
    @builtin(.BitwiseNotOperatorMethod)
    func bitwiseNot() -> Output
}

/// Protocol for types that support left bit shift (<<).
/// Default Rhs is lang.i64 because type defaults must be resolvable at parse time.
@builtin(.ShiftLeftOperatorProtocol)
public protocol LeftShift[Rhs = lang.i64] {
    type Output

    /// Shifts this value's bits left by the given count.
    @builtin(.ShiftLeftOperatorMethod)
    func shiftLeft(by count: Rhs) -> Output
}

/// Protocol for types that support right bit shift (>>).
/// Default Rhs is lang.i64 because type defaults must be resolvable at parse time.
@builtin(.ShiftRightOperatorProtocol)
public protocol RightShift[Rhs = lang.i64] {
    type Output

    /// Shifts this value's bits right by the given count.
    @builtin(.ShiftRightOperatorMethod)
    func shiftRight(by count: Rhs) -> Output
}
