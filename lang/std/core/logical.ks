// Logical operator protocols
// Kestrel uses keyword-style logical operators for clarity
// Note: Method names use 'logical*' prefix because 'and', 'or', 'not' are keywords

module std.core

@builtin(.LogicalAndOperatorProtocol)
public protocol And[Rhs = Self] {
    type Output

    // Takes a closure for short-circuit evaluation:
    // The closure is only called if self is truthy
    @builtin(.LogicalAndOperatorMethod)
    func logicalAnd(other: () -> Rhs) -> Output
}

@builtin(.LogicalOrOperatorProtocol)
public protocol Or[Rhs = Self] {
    type Output

    // Takes a closure for short-circuit evaluation:
    // The closure is only called if self is falsy
    @builtin(.LogicalOrOperatorMethod)
    func logicalOr(other: () -> Rhs) -> Output
}

@builtin(.LogicalNotOperatorProtocol)
public protocol Not {
    type Output

    @builtin(.LogicalNotOperatorMethod)
    func logicalNot() -> Output
}

// Protocol for types that can be used as boolean conditions in if/while
@builtin(.BooleanConditional)
public protocol BooleanConditional {
    func boolValue() -> lang.i1
}
