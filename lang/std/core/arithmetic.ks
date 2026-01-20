// Arithmetic operator protocols

module std.core

@builtin(.AddOperatorProtocol)
public protocol Addable[Rhs = Self] {
    type Output

    @builtin(.AddOperatorMethod)
    func add(other: Rhs) -> Output
}

@builtin(.SubtractOperatorProtocol)
public protocol Subtractable[Rhs = Self] {
    type Output

    @builtin(.SubtractOperatorMethod)
    func subtract(other: Rhs) -> Output
}

@builtin(.MultiplyOperatorProtocol)
public protocol Multipliable[Rhs = Self] {
    type Output

    @builtin(.MultiplyOperatorMethod)
    func multiply(other: Rhs) -> Output
}

@builtin(.DivideOperatorProtocol)
public protocol Divisible[Rhs = Self] {
    type Output

    @builtin(.DivideOperatorMethod)
    func divide(other: Rhs) -> Output
}

@builtin(.ModuloOperatorProtocol)
public protocol Modulo[Rhs = Self] {
    type Output

    @builtin(.ModuloOperatorMethod)
    func modulo(other: Rhs) -> Output
}

@builtin(.NegateOperatorProtocol)
public protocol Negatable {
    type Output

    @builtin(.NegateOperatorMethod)
    func negate() -> Output
}
