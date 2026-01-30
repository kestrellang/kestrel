// Error protocol and control flow types
// These types enable Kestrel's error handling and early return mechanisms.

module std.core

/// The fundamental type for early return semantics.
/// Used internally by the try operator and error propagation.
/// Continue indicates normal flow with the contained value.
/// Break indicates early return/error with the contained value.
@builtin(.ControlFlowEnum)
public enum ControlFlow[C, B] {
    case Continue(C)
    case Break(B)
}

/// Protocol that enables the `try expr` syntax.
/// Types conforming to Tryable can be used with the try operator,
/// which extracts the success value or propagates the error.
@builtin(.TryableProtocol)
public protocol Tryable {
    /// The type produced on success.
    type Output
    /// The type propagated on failure.
    type Early

    /// Extracts the value or signals early return.
    /// Returns Continue(value) on success or Break(early) on failure.
    @builtin(.TryExtractMethod)
    func tryExtract() -> ControlFlow[Output, Early]
}

/// Protocol that enables early return propagation.
/// Types conforming to FromResidual can be constructed from an early return value,
/// allowing error types to propagate through function boundaries.
@builtin(.FromResidualProtocol)
public protocol FromResidual[Early] {
    /// Creates an instance from an early return/error value.
    @builtin(.FromResidualMethod)
    static func fromResidual(residual: Early) -> Self
}

/// Protocol that enables `return value` in result-returning contexts.
/// Allows wrapping a success value into the return type.
public protocol Returnable[Output] {
    /// Creates an instance from a success value.
    static func fromOutput(value: Output) -> Self
}
