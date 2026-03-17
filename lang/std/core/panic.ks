// Panic: abort the process with a message.

module std.core

/// Abort the process with a message.
public func fatalError(message: String) {
    lang.panic_unwind("fatal error")
}
