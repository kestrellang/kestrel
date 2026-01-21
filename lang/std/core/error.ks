// Error protocol and control flow types

module std.core

// ControlFlow enum - the fundamental type for early return semantics
@builtin(.ControlFlowEnum)
public enum ControlFlow[C, B] {
    case Continue(C)
    case Break(B)
}

// Tryable - enables `try expr`
@builtin(.TryableProtocol)
public protocol Tryable {
    type Output
    type Early

    @builtin(.TryExtractMethod)
    func tryExtract() -> ControlFlow[Output, Early]
}

// FromResidual - enables early return propagation
@builtin(.FromResidualProtocol)
public protocol FromResidual[Early] {
    @builtin(.FromResidualMethod)
    static func fromResidual(residual: Early) -> Self
}

// Returnable - enables `return value` in result-returning contexts
public protocol Returnable[Output] {
    static func fromOutput(value: Output) -> Self
}
