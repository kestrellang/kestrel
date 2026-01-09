// Comparison operator protocols
// These are raw operator protocols with flexible return types.
// Semantic protocols (Equatable, Comparable) provide Bool-returning implementations.

module std.ops

// TODO: Add back 
//@operator(==)
public protocol Equal[Rhs = Self] {
    type Output
    func eq(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(!=)
public protocol NotEqual[Rhs = Self] {
    type Output
    func ne(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(<)
public protocol Less[Rhs = Self] {
    type Output
    func lt(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(<=)
public protocol LessOrEqual[Rhs = Self] {
    type Output
    func le(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(>)
public protocol Greater[Rhs = Self] {
    type Output
    func gt(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(>=)
public protocol GreaterOrEqual[Rhs = Self] {
    type Output
    func ge(other: Rhs) -> Output
}
