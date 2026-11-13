// skip: included file, not a standalone test
module Prelude

@builtin(.ControlFlowEnum)
public enum ControlFlow[C, B] {
    @builtin(.ControlFlowContinueCase)
    case Continue(C)
    @builtin(.ControlFlowBreakCase)
    case Break(B)
}

@builtin(.TryableProtocol)
public protocol Tryable {
    type Output
    type Early

    @builtin(.TryExtractMethod)
    func tryExtract() -> ControlFlow[Output, Early]
}

@builtin(.FromResidualProtocol)
public protocol FromResidual[Early] {
    @builtin(.FromResidualMethod)
    static func fromResidual(residual: Early) -> Self
}
