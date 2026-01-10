// Bitwise operator protocols

module std.ops

// TODO: Add back 
//@operator(&)
public protocol BitwiseAnd[Rhs = Self] {
    type Output
    func bitwiseAnd(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(|)
public protocol BitwiseOr[Rhs = Self] {
    type Output
    func bitwiseOr(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(^)
public protocol BitwiseXor[Rhs = Self] {
    type Output
    func bitwiseXor(other: Rhs) -> Output
}

// TODO: Add back 
////@operator(prefix ~)
public protocol BitwiseNot {
    type Output
    func bitwiseNot() -> Output
}

// TODO: Add back
//@operator(<<)
// Note: Default Rhs is lang.i64 (not Int type alias) because type defaults
// must be resolvable at parse time before type aliases are available.
public protocol LeftShift[Rhs = lang.i64] {
    type Output
    func shiftLeft(by count: Rhs) -> Output
}

// TODO: Add back
//@operator(>>)
// Note: Default Rhs is lang.i64 (not Int type alias) because type defaults
// must be resolvable at parse time before type aliases are available.
public protocol RightShift[Rhs = lang.i64] {
    type Output
    func shiftRight(by count: Rhs) -> Output
}
