// Logical operator protocols
// Kestrel uses keyword-style logical operators (and, or, not) for clarity.
// Method names use 'logical*' prefix because 'and', 'or', 'not' are keywords.

module std.core

/// Raw protocol backing the `and` keyword operator.
///
/// The `other` operand is a thunk so that conformers can short-circuit:
/// the right-hand side must not be evaluated when `self` is falsy. The
/// stdlib implementations on `Bool` and the optional types all honour
/// this; user implementations should too.
///
/// # Examples
///
/// ```
/// true and false        // false
/// true and { true }     // true (closure form, mostly internal)
/// ```
@builtin(.LogicalAndOperatorProtocol)
public protocol And[Rhs = Self] {
    type Output

    /// Returns `self and other()`. The closure runs only if needed.
    @builtin(.LogicalAndOperatorMethod)
    func logicalAnd(other: () -> Rhs) -> Output
}

/// Raw protocol backing the `or` keyword operator.
///
/// As with `And`, `other` is a thunk so the right-hand side can be skipped
/// when `self` already determines the result.
@builtin(.LogicalOrOperatorProtocol)
public protocol Or[Rhs = Self] {
    type Output

    /// Returns `self or other()`. The closure runs only if needed.
    @builtin(.LogicalOrOperatorMethod)
    func logicalOr(other: () -> Rhs) -> Output
}

/// Raw protocol backing the `not` keyword operator.
@builtin(.LogicalNotOperatorProtocol)
public protocol Not {
    type Output

    /// Returns `not self`.
    @builtin(.LogicalNotOperatorMethod)
    func logicalNot() -> Output
}

/// Protocol for types that may appear directly in `if`, `while`, and other
/// boolean contexts.
///
/// `Bool` is the canonical conformer. The method returns the primitive
/// `lang.i1` rather than `Bool` to avoid a circular dependency between the
/// conditional lowering and `Bool` itself.
@builtin(.BooleanConditional)
public protocol BooleanConditional {
    /// Returns the underlying truth value as a primitive `i1`.
    func boolValue() -> lang.i1
}
