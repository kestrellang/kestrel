// I/O Error types

module std.io.error

import std.num.(Int32, Int64)
import std.text.(String)
import std.result.(Result)
import std.io.libc

// ============================================================================
// I/O ERROR
// ============================================================================

/// I/O error wrapping a POSIX `errno` code.
///
/// Returned in the `Err` arm of every `Result` produced by `Read`, `Write`,
/// and the `File`/`stdio` helpers. The `description()` method maps a small
/// set of common codes to readable strings — for full coverage call
/// `errno()` and look the value up via the platform's `strerror`. The
/// freestanding `notFound()`, `permissionDenied()`, etc. constructors below
/// are convenience shorthands for the most-used codes.
///
/// # Examples
///
/// ```
/// match File.open("missing.txt") {
///     .Ok(f) => use(f),
///     .Err(e) => print("error: " + e.description())
/// }
/// ```
public struct Error {
    var code: Int32

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name From Code
    /// Wraps a raw POSIX error code.
    public init(code: Int32) {
        self.code = code
    }

    /// Snapshots the current value of the platform's `errno` thread-local.
    /// Call immediately after a failed libc call — any other libc activity
    /// in between can clobber the value.
    public static func last() -> Error {
        Error(libc.errno())
    }

    // ========================================================================
    // ERROR INFORMATION
    // ========================================================================

    /// Returns a short human-readable phrase for a handful of common codes
    /// (ENOENT, EACCES, EPIPE, etc.). Unknown codes yield `"unknown error"`;
    /// for full coverage use `errno()` with a platform `strerror`.
    public func description() -> String {
        // Convert to Int64 for match (integer literals default to Int64)
        let code64 = Int64(from: self.code);
        match code64 {
            1 => "operation not permitted",
            2 => "no such file or directory",
            4 => "interrupted",
            5 => "i/o error",
            9 => "bad file descriptor",
            11 => "would block",
            12 => "out of memory",
            13 => "permission denied",
            17 => "file exists",
            20 => "not a directory",
            21 => "is a directory",
            22 => "invalid argument",
            28 => "no space left",
            32 => "broken pipe",
            _ => "unknown error"
        }
    }

    /// The raw POSIX error code. Use for programmatic dispatch (`if e.errno() == 13 { … }`).
    public func errno() -> Int32 {
        self.code
    }
}

// ============================================================================
// COMMON ERROR CONSTRUCTORS
// ============================================================================

/// `ENOENT` — the path does not exist.
public func notFound() -> Error { Error(2) }

/// `EACCES` — caller lacks permission for the operation.
public func permissionDenied() -> Error { Error(13) }

/// `EEXIST` — the path already exists (e.g. `O_CREAT | O_EXCL`).
public func alreadyExists() -> Error { Error(17) }

/// `EINVAL` — invalid argument to a libc call.
public func invalidInput() -> Error { Error(22) }

/// `EAGAIN` — non-blocking call would have blocked.
public func wouldBlock() -> Error { Error(11) }

/// `EINTR` — operation interrupted by a signal.
public func interrupted() -> Error { Error(4) }

/// `EPIPE` — write to a pipe with no reader.
public func brokenPipe() -> Error { Error(32) }
