// Panic: abort the process with a message.

module std.core

/// Aborts the process with `message`.
///
/// Returns `!` (the never type), so the compiler treats any code after a
/// `fatalError` call as unreachable. Use sparingly — almost every "this
/// should be impossible" branch is better expressed as a `Result` error or
/// a precondition check, because `fatalError` produces no recovery
/// opportunity for the caller.
///
/// # Examples
///
/// ```
/// let mode = readMode();
/// match mode {
///     .Read => doRead(),
///     .Write => doWrite(),
///     _ => fatalError("unsupported mode")
/// }
/// ```
public func fatalError(message: String) -> ! {
    lang.panic_unwind("fatal error")
}
