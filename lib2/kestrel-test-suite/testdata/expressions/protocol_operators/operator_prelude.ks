// skip: included file, not a standalone test
module Prelude

// Arithmetic
@builtin(.AddOperatorProtocol)
public protocol AddOperatorProtocol {
    @builtin(.AddOperatorMethod)
    func add(rhs: Self) -> Self
}

@builtin(.SubtractOperatorProtocol)
public protocol SubtractOperatorProtocol {
    @builtin(.SubtractOperatorMethod)
    func subtract(rhs: Self) -> Self
}

@builtin(.MultiplyOperatorProtocol)
public protocol MultiplyOperatorProtocol {
    @builtin(.MultiplyOperatorMethod)
    func multiply(rhs: Self) -> Self
}

@builtin(.DivideOperatorProtocol)
public protocol DivideOperatorProtocol {
    @builtin(.DivideOperatorMethod)
    func divide(rhs: Self) -> Self
}

@builtin(.ModuloOperatorProtocol)
public protocol ModuloOperatorProtocol {
    @builtin(.ModuloOperatorMethod)
    func modulo(rhs: Self) -> Self
}

@builtin(.NegateOperatorProtocol)
public protocol NegateOperatorProtocol {
    @builtin(.NegateOperatorMethod)
    func negate() -> Self
}

// Comparison
@builtin(.EqualsOperatorProtocol)
public protocol EqualsOperatorProtocol {
    @builtin(.EqualsOperatorMethod)
    func equals(rhs: Self) -> lang.i1
}

@builtin(.NotEqualsOperatorProtocol)
public protocol NotEqualsOperatorProtocol {
    @builtin(.NotEqualsOperatorMethod)
    func notEquals(rhs: Self) -> lang.i1
}

@builtin(.LessThanOperatorProtocol)
public protocol LessThanOperatorProtocol {
    @builtin(.LessThanOperatorMethod)
    func lessThan(rhs: Self) -> lang.i1
}

@builtin(.GreaterThanOperatorProtocol)
public protocol GreaterThanOperatorProtocol {
    @builtin(.GreaterThanOperatorMethod)
    func greaterThan(rhs: Self) -> lang.i1
}

@builtin(.LessOrEqualOperatorProtocol)
public protocol LessOrEqualOperatorProtocol {
    @builtin(.LessOrEqualOperatorMethod)
    func lessThanOrEqual(rhs: Self) -> lang.i1
}

@builtin(.GreaterOrEqualOperatorProtocol)
public protocol GreaterOrEqualOperatorProtocol {
    @builtin(.GreaterOrEqualOperatorMethod)
    func greaterThanOrEqual(rhs: Self) -> lang.i1
}

// Bitwise
@builtin(.BitwiseAndOperatorProtocol)
public protocol BitwiseAndOperatorProtocol {
    @builtin(.BitwiseAndOperatorMethod)
    func bitwiseAnd(rhs: Self) -> Self
}

@builtin(.BitwiseOrOperatorProtocol)
public protocol BitwiseOrOperatorProtocol {
    @builtin(.BitwiseOrOperatorMethod)
    func bitwiseOr(rhs: Self) -> Self
}

@builtin(.BitwiseXorOperatorProtocol)
public protocol BitwiseXorOperatorProtocol {
    @builtin(.BitwiseXorOperatorMethod)
    func bitwiseXor(rhs: Self) -> Self
}

@builtin(.ShiftLeftOperatorProtocol)
public protocol ShiftLeftOperatorProtocol {
    @builtin(.ShiftLeftOperatorMethod)
    func shiftLeft(rhs: Self) -> Self
}

@builtin(.ShiftRightOperatorProtocol)
public protocol ShiftRightOperatorProtocol {
    @builtin(.ShiftRightOperatorMethod)
    func shiftRight(rhs: Self) -> Self
}

@builtin(.BitwiseNotOperatorProtocol)
public protocol BitwiseNotOperatorProtocol {
    @builtin(.BitwiseNotOperatorMethod)
    func bitwiseNot() -> Self
}

// Logical
@builtin(.LogicalNotOperatorProtocol)
public protocol LogicalNotOperatorProtocol {
    @builtin(.LogicalNotOperatorMethod)
    func logicalNot() -> lang.i1
}
