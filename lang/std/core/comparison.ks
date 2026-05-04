// Comparison operator protocols
// These are raw operator protocols with flexible return types.
// Semantic protocols (Equatable, Comparable) provide Bool-returning implementations.

module std.core

/// Raw protocol backing the `==` operator.
///
/// Most user code should conform to `Equatable` instead, which conforms to
/// `Equal[Self]` automatically with `Output = Bool`. Implement `Equal` directly
/// only when you need a non-Bool result (e.g. lifting equality into a vector
/// type that returns a mask).
///
/// # Examples
///
/// ```
/// 1 == 1   // true
/// "a" == "b"  // false
/// ```
@builtin(.EqualsOperatorProtocol)
public protocol Equal[Other = Self] {
    type Output

    /// Returns the equality result as `Output` — typically `Bool`.
    @builtin(.EqualsOperatorMethod)
    func isEqual(to other: Other) -> Output
}

/// Raw protocol backing the `!=` operator.
///
/// `Equatable` provides a default `isNotEqual` derived from `isEqual`, so
/// conforming to `Equatable` is enough for both `==` and `!=`.
@builtin(.NotEqualsOperatorProtocol)
public protocol NotEqual[Other = Self] {
    type Output

    /// Returns the inequality result as `Output` — typically `Bool`.
    @builtin(.NotEqualsOperatorMethod)
    func isNotEqual(to other: Other) -> Output
}

/// Raw protocol backing the `<` operator.
///
/// `Comparable` derives `Less`, `LessOrEqual`, `Greater`, `GreaterOrEqual` from
/// a single `compare()` method, so prefer conforming to `Comparable` for
/// totally-ordered types.
@builtin(.LessThanOperatorProtocol)
public protocol Less[Other = Self] {
    type Output

    /// Returns the less-than result as `Output` — typically `Bool`.
    @builtin(.LessThanOperatorMethod)
    func lessThan(other: Other) -> Output
}

/// Raw protocol backing the `<=` operator. See `Less` for guidance.
@builtin(.LessOrEqualOperatorProtocol)
public protocol LessOrEqual[Other = Self] {
    type Output

    /// Returns the less-than-or-equal result as `Output` — typically `Bool`.
    @builtin(.LessOrEqualOperatorMethod)
    func lessThanOrEqual(other: Other) -> Output
}

/// Raw protocol backing the `>` operator. See `Less` for guidance.
@builtin(.GreaterThanOperatorProtocol)
public protocol Greater[Other = Self] {
    type Output

    /// Returns the greater-than result as `Output` — typically `Bool`.
    @builtin(.GreaterThanOperatorMethod)
    func greaterThan(other: Other) -> Output
}

/// Raw protocol backing the `>=` operator. See `Less` for guidance.
@builtin(.GreaterOrEqualOperatorProtocol)
public protocol GreaterOrEqual[Other = Self] {
    type Output

    /// Returns the greater-than-or-equal result as `Output` — typically `Bool`.
    @builtin(.GreaterOrEqualOperatorMethod)
    func greaterThanOrEqual(other: Other) -> Output
}
