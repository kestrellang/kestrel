// Logical operator protocols
// Kestrel uses keyword-style logical operators for clarity
// Note: Method names use 'logical*' prefix because 'and', 'or', 'not' are keywords

module std.ops

@builtin(.LogicalAndOperatorProtocol)
public protocol And[Rhs = Self] {
    type Output

    @builtin(.LogicalAndOperatorMethod)
    func logicalAnd(other: Rhs) -> Output
}

@builtin(.LogicalOrOperatorProtocol)
public protocol Or[Rhs = Self] {
    type Output

    @builtin(.LogicalOrOperatorMethod)
    func logicalOr(other: Rhs) -> Output
}

@builtin(.LogicalNotOperatorProtocol)
public protocol Not {
    type Output

    @builtin(.LogicalNotOperatorMethod)
    func logicalNot() -> Output
}
