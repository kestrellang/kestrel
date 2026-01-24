// Null coalescing operator protocol
// The ?? operator provides a concise way to unwrap optionals with a default value

module std.core

@builtin(.CoalesceOperatorProtocol)
public protocol Coalesce[Default] {
    type Output

    // Takes a closure for short-circuit evaluation:
    // The closure is only called if self is None/null
    @builtin(.CoalesceOperatorMethod)
    func coalesce(default: () -> Default) -> Output
}
