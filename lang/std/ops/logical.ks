// Logical operator protocols
// Kestrel uses keyword-style logical operators for clarity
// Note: Method names use 'logical*' prefix because 'and', 'or', 'not' are keywords

module std.ops

// TODO: Add back
//@operator(and)
public protocol And[Rhs = Self] {
    type Output
    func logicalAnd(other: Rhs) -> Output
}

// TODO: Add back
//@operator(or)
public protocol Or[Rhs = Self] {
    type Output
    func logicalOr(other: Rhs) -> Output
}

// TODO: Add back
//@operator(not)
public protocol Not {
    type Output
    func logicalNot() -> Output
}
