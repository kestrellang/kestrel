// Error protocol and related types

module std.result

import std.core.(Equatable, Bool)
import std.text.(String)

public protocol Error {
    func description() -> String
}

// ControlFlow enum - the fundamental type for early return semantics
@builtin(.ControlFlowEnum)
public enum ControlFlow[C, B] {
    case Continue(C)
    case Break(B)
}

extend ControlFlow[C, B]: Equatable
    where C: Equatable, B: Equatable
{
    public func equals(other: ControlFlow[C, B]) -> Bool {
        match (self, other) {
            (.Continue(a), .Continue(b)) => a == b,
            (.Break(a), .Break(b)) => a == b,
            _ => false
        }
    }
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

// Convertible - for error type conversions
public protocol Convertible[From] {
    init(from value: From)
}
