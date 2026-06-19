// Panic: abort the process with a message.

module std.core

import std.text.(String)
import std.io.stdio.(eprintln)

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
    // Best-effort: print the message to stderr via the normal I/O path, then
    // trap. The Result is intentionally discarded — there is no recovery and
    // the trap is unconditional. `lang.panic()` is an argument-free diverging
    // intrinsic (a bare CPU trap); the message surfaces here, not in codegen.
    eprintln("fatal error: \(message)");
    lang.panic()
}
