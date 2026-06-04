// Error protocol and control flow types
// These types enable Kestrel's error handling and early return mechanisms.

module std.core

/// The two-state result of a `tryExtract()` call: keep going with a value, or
/// short-circuit out of the current function with an early-return payload.
///
/// Conceptually `Either`-shaped, but the names are deliberately
/// control-flow flavoured because that is what the compiler does with
/// them — `Continue` flows to the next instruction, `Break` lowers into a
/// branch back to the function's epilogue via `FromResidual`.
@builtin(.ControlFlowEnum)
public enum ControlFlow[C, B]: not Copyable {
    /// Normal flow — carries the value to use as the operator result.
    case Continue(C)
    /// Residual-return flow — carries the residual to propagate via `FromResidual`.
    case Break(B)
}

/// Move-only by default so `tryExtract()` can carry a non-Copyable success or
/// residual payload; bit-copyable only when both payloads are Copyable.
extend ControlFlow[C, B]: Copyable where C: Copyable, B: Copyable { }

/// Protocol enabling the `try expr` operator.
///
/// `Output` is the success value the operator yields; `Residual` is the
/// "residual" — typically an `Err` variant, a `None`, or a typed error —
/// that gets propagated. The compiler lowers `try x` to roughly
/// `match x.tryExtract() { .Continue(v) => v, .Break(r) => return Self.fromResidual(r) }`,
/// which is why the enclosing function's return type must conform to
/// `FromResidual[Residual]`.
///
/// # Examples
///
/// ```
/// // Optional and Result both conform; `try` chains them seamlessly.
/// func parseAndDouble(s: String) -> Int64? {
///     let n = try Int64.parse(s);    // .None short-circuits the whole function
///     .Some(n * 2)
/// }
/// ```
@builtin(.TryableProtocol)
public protocol Tryable {
    /// The value produced by `try expr` on success.
    type Output
    /// The residual carried out of `try expr` on failure.
    type Residual

    /// Splits `self` into the success value or the early-return residual.
    @builtin(.TryExtractMethod)
    consuming func tryExtract() -> ControlFlow[Output, Residual]
}

/// Protocol that lets a return type absorb a `try`-propagated residual.
///
/// Implement when your error/optional type should be reachable via `try`
/// from another type with a different residual. For example, `Result[T, E]`
/// implements `FromResidual[E]` so that `try someResult` inside a function
/// returning `Result[T, E]` rebuilds the failure.
@builtin(.FromResidualProtocol)
public protocol FromResidual[Residual] {
    /// Builds an instance carrying `residual` as its failure payload.
    @builtin(.FromResidualMethod)
    static func fromResidual(residual: Residual) -> Self
}

/// Protocol enabling implicit promotion of a bare value into a wrapping type.
///
/// Used by the compiler so a function returning `T?` can `return v` and have
/// the value lifted into `.Some(v)`, etc. Not part of the public API; the
/// stdlib wires it up for `Optional` and `Result`.
@builtin(.FromValueProtocol)
protocol FromValue[Output] {
    /// Lifts `value` into an instance of the conforming type.
    /// Takes `value` `consuming`: the promotion site transfers ownership, so
    /// `from` moves it into the wrapper rather than cloning a borrow and leaking
    /// the original (the kestrel-wall per-GET query-result leak).
    @builtin(.FromValueMethod)
    static func from(consuming value: Output) -> Self
}
