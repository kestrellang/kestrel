// Ending the process.

module std.system

import std.numeric.(Int32)

// `ExitCode` and `Exitable` are re-exported from std.os, which owns the
// `@builtin` hooks the `@main` wrapper uses. They move here when std.os goes.
import std.os.(ExitCode, Exitable)

/// Ends the process with `code`, the same type `@main` returns.
///
/// Runs `atexit` handlers, which flush C stdio buffers and std.streams'
/// standard output buffer. Kestrel values still in scope are **not**
/// deinitialized, so flush any other buffered writers first.
///
/// # Examples
///
/// ```
/// if not configFound {
///     eprintln("no config file");
///     exit(2)
/// }
/// ```
public func exit(code: ExitCode) -> ! {
    native_exit(Int32(from: code.value));
    loop {}
}

/// Ends the process immediately with `SIGABRT`, without running `atexit`
/// handlers or flushing anything. For states where continuing, or even
/// cleaning up, could do harm. Use `exit` for ordinary failures.
public func abort() -> ! {
    native_abort();
    loop {}
}
