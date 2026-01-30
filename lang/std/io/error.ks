// I/O Error types

module std.io.error

import std.num.(Int32, Int64)
import std.text.(String)
import std.result.(Result)
import std.io.libc

// ============================================================================
// I/O ERROR
// ============================================================================

/// Represents an I/O error with a POSIX error code.
public struct Error {
    var code: Int32

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates an error from an error code.
    public init(code: Int32) {
        self.code = code
    }

    /// Creates an error from the current errno value.
    public static func last() -> Error {
        Error(libc.errno())
    }

    // ========================================================================
    // ERROR INFORMATION
    // ========================================================================

    /// Returns a human-readable description of the error.
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

    /// Returns the raw POSIX error code.
    public func errno() -> Int32 {
        self.code
    }
}

// ============================================================================
// COMMON ERROR CONSTRUCTORS
// ============================================================================

/// Creates a "not found" error (ENOENT).
public func notFound() -> Error { Error(2) }

/// Creates a "permission denied" error (EACCES).
public func permissionDenied() -> Error { Error(13) }

/// Creates an "already exists" error (EEXIST).
public func alreadyExists() -> Error { Error(17) }

/// Creates an "invalid input" error (EINVAL).
public func invalidInput() -> Error { Error(22) }

/// Creates a "would block" error (EAGAIN).
public func wouldBlock() -> Error { Error(11) }

/// Creates an "interrupted" error (EINTR).
public func interrupted() -> Error { Error(4) }

/// Creates a "broken pipe" error (EPIPE).
public func brokenPipe() -> Error { Error(32) }
