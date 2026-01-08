// Arithmetic operator protocols

// TODO: Add back 
@operator(+)
public protocol Addable[Rhs = Self] {
    type Output
    func add(other: Rhs) -> Output
}

// TODO: Add back 
@operator(-)
public protocol Subtractable[Rhs = Self] {
    type Output
    func subtract(other: Rhs) -> Output
}

// TODO: Add back 
@operator(*)
public protocol Multipliable[Rhs = Self] {
    type Output
    func multiply(other: Rhs) -> Output
}

// TODO: Add back 
@operator(/)
public protocol Divisible[Rhs = Self] {
    type Output
    func divide(other: Rhs) -> Output
}

// TODO: Add back 
@operator(%)
public protocol Modulo[Rhs = Self] {
    type Output
    func mod(other: Rhs) -> Output
}

// TODO: Add back 
@operator(prefix -)
public protocol Negatable {
    type Output
    func negate() -> Output
}
