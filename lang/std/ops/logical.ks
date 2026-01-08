// Logical operator protocols
// Kestrel uses keyword-style logical operators for clarity

// TODO: Add back 
@operator(and)
public protocol And[Rhs = Self] {
    type Output
    func and(other: Rhs) -> Output
}

// TODO: Add back 
@operator(or)
public protocol Or[Rhs = Self] {
    type Output
    func or(other: Rhs) -> Output
}

// TODO: Add back 
@operator(not)
public protocol Not {
    type Output
    func not() -> Output
}
