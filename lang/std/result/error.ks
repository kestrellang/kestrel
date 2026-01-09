// Error protocol and related types

module std.result

public protocol Error {
    func description() -> String
}

// Residual enum - the fundamental type for early return semantics
public enum Residual[Output, Early] {
    case Output(Output)  // continue with value
    case Early(Early)    // break/return early with value
}

extension Residual[Output, Early]: Equatable
    where Output: Equatable, Early: Equatable
{
    public func equals(other: Residual[Output, Early]) -> Bool {
        match (self, other) {
            (.Output(let a), .Output(let b)) => a == b,
            (.Early(let a), .Early(let b)) => a == b,
            _ => false
        }
    }
}

// Tryable - enables `try expr`
public protocol Tryable[Output, Early] {
    func tryExtract() -> Residual[Output, Early]
}

// Throwable - enables `throw error`
public protocol Throwable[Early] {
    static func fromEarly(value: Early) -> Self
}

// Returnable - enables `return value`
public protocol Returnable[Output] {
    static func fromOutput(value: Output) -> Self
}

// Convertible - for error type conversions
public protocol Convertible[From] {
    init(from value: From)
}

// Residual is also Tryable (identity)
extension Residual[Output, Early]: Tryable[Output, Early] {
    public func tryExtract() -> Residual[Output, Early] {
        self
    }
}
