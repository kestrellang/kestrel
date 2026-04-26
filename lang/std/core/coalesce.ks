// Null coalescing operator protocol
// The ?? operator provides a concise way to unwrap optionals with a default value.

module std.core

/// Raw protocol backing the `??` operator.
///
/// Implemented by `Optional[T]` (with `Default = T`, `Output = T`) and by
/// `Result[T, E]` (with `Default = T`, `Output = T`). The operand is a
/// thunk so the default expression is only evaluated when needed — this
/// matters when the default has side effects or is expensive to compute.
///
/// # Examples
///
/// ```
/// let name: String? = .None;
/// name ?? "anonymous"           // "anonymous"
///
/// let cached: String? = .Some("hi");
/// cached ?? expensiveLookup()   // "hi" — expensiveLookup() not called
/// ```
@builtin(.CoalesceOperatorProtocol)
public protocol Coalesce[Default] {
    type Output

    /// Returns the contained value, or the result of `default()` if absent.
    @builtin(.CoalesceOperatorMethod)
    func coalesce(default: () -> Default) -> Output
}
