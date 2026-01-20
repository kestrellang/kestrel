// Comparison operator protocols
// These are raw operator protocols with flexible return types.
// Semantic protocols (Equatable, Comparable) provide Bool-returning implementations.

module std.core

@builtin(.EqualsOperatorProtocol)
public protocol Equal[Rhs = Self] {
    type Output

    @builtin(.EqualsOperatorMethod)
    func equals(other: Rhs) -> Output
}

@builtin(.NotEqualsOperatorProtocol)
public protocol NotEqual[Rhs = Self] {
    type Output

    @builtin(.NotEqualsOperatorMethod)
    func notEquals(other: Rhs) -> Output
}

@builtin(.LessThanOperatorProtocol)
public protocol Less[Rhs = Self] {
    type Output

    @builtin(.LessThanOperatorMethod)
    func lessThan(other: Rhs) -> Output
}

@builtin(.LessOrEqualOperatorProtocol)
public protocol LessOrEqual[Rhs = Self] {
    type Output

    @builtin(.LessOrEqualOperatorMethod)
    func lessThanOrEqual(other: Rhs) -> Output
}

@builtin(.GreaterThanOperatorProtocol)
public protocol Greater[Rhs = Self] {
    type Output

    @builtin(.GreaterThanOperatorMethod)
    func greaterThan(other: Rhs) -> Output
}

@builtin(.GreaterOrEqualOperatorProtocol)
public protocol GreaterOrEqual[Rhs = Self] {
    type Output

    @builtin(.GreaterOrEqualOperatorMethod)
    func greaterThanOrEqual(other: Rhs) -> Output
}
