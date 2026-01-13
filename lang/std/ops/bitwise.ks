// Bitwise operator protocols

module std.ops

@builtin(.BitwiseAndOperatorProtocol)
public protocol BitwiseAnd[Rhs = Self] {
    type Output

    @builtin(.BitwiseAndOperatorMethod)
    func bitwiseAnd(other: Rhs) -> Output
}

@builtin(.BitwiseOrOperatorProtocol)
public protocol BitwiseOr[Rhs = Self] {
    type Output

    @builtin(.BitwiseOrOperatorMethod)
    func bitwiseOr(other: Rhs) -> Output
}

@builtin(.BitwiseXorOperatorProtocol)
public protocol BitwiseXor[Rhs = Self] {
    type Output

    @builtin(.BitwiseXorOperatorMethod)
    func bitwiseXor(other: Rhs) -> Output
}

@builtin(.BitwiseNotOperatorProtocol)
public protocol BitwiseNot {
    type Output

    @builtin(.BitwiseNotOperatorMethod)
    func bitwiseNot() -> Output
}

// Note: Default Rhs is lang.i64 (not Int type alias) because type defaults
// must be resolvable at parse time before type aliases are available.
@builtin(.ShiftLeftOperatorProtocol)
public protocol LeftShift[Rhs = lang.i64] {
    type Output

    @builtin(.ShiftLeftOperatorMethod)
    func shiftLeft(by count: Rhs) -> Output
}

// Note: Default Rhs is lang.i64 (not Int type alias) because type defaults
// must be resolvable at parse time before type aliases are available.
@builtin(.ShiftRightOperatorProtocol)
public protocol RightShift[Rhs = lang.i64] {
    type Output

    @builtin(.ShiftRightOperatorMethod)
    func shiftRight(by count: Rhs) -> Output
}
