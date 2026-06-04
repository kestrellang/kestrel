// Force-unwrap operator protocol
// The postfix `!` operator extracts the wrapped value, trapping if absent.

module std.core

/// Raw protocol backing the postfix `!` (force-unwrap) operator.
///
/// Implemented by `Optional[T]` (with `Output = T`): `value!` yields the
/// contained value, or aborts the process via `fatalError` when the value is
/// `.None`. Use `!` only where the absence of a value is genuinely a
/// programming error — prefer `if let`, `??`, or `try` when `.None` is a
/// recoverable case.
///
/// # Examples
///
/// ```
/// let port: Int64? = .Some(8080);
/// let p = port!               // 8080
///
/// let missing: Int64? = .None;
/// missing!                    // PANIC
/// ```
@builtin(.ForceUnwrapOperatorProtocol)
public protocol ForceUnwrap {
    type Output

    /// Returns the contained value, or traps if it is absent.
    @builtin(.ForceUnwrapOperatorMethod)
    consuming func forceUnwrap() -> Output
}
