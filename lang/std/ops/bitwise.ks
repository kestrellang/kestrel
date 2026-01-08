// Bitwise operator protocols

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
public protocol LeftShift[Rhs = Int] {
    type Output
    func shiftLeft(by count: Rhs) -> Output
}

// TODO: Add back 
//@operator(>>)
public protocol RightShift[Rhs = Int] {
    type Output
    func shiftRight(by count: Rhs) -> Output
}
