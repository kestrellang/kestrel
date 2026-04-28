// I/O Error types

module std.io.error

import std.core.(Bool)
import std.numeric.(Int32, Int64)
import std.text.(String)
import std.result.(Result)
import std.io.libc

// ============================================================================
// I/O ERROR KIND
// ============================================================================

/// Categorical classification of an I/O error.
///
/// `IoError` carries one of these alongside its raw `errno`. The named
/// variants cover the common categories applications dispatch on; everything
/// else falls into `.Other` carrying the original POSIX code so no
/// information is lost. Built from a code via `IoErrorKind.fromErrno(code:)`,
/// or matched directly in error-handling code:
///
/// ```
/// match e.kind {
///     .NotFound          => createDefault(),
///     .PermissionDenied  => promptForElevation(),
///     .Other(c)          => log("unhandled errno: " + c.toString())
/// }
/// ```
public enum IoErrorKind {
    /// `ENOENT` — the path does not exist.
    case NotFound
    /// `EACCES` — caller lacks permission for the operation.
    case PermissionDenied
    /// `EEXIST` — the path already exists (e.g. `O_CREAT | O_EXCL`).
    case AlreadyExists
    /// `EINVAL` — invalid argument to a libc call.
    case InvalidInput
    /// `EAGAIN` — non-blocking call would have blocked.
    case WouldBlock
    /// `EINTR` — operation interrupted by a signal.
    case Interrupted
    /// `EPIPE` — write to a pipe with no reader.
    case BrokenPipe
    /// `ENOMEM` — kernel allocation failed.
    case OutOfMemory
    /// `ENOTDIR` — a path component is not a directory.
    case NotADirectory
    /// `EISDIR` — operation expected a file but got a directory.
    case IsADirectory
    /// `ENOSPC` — no space left on device.
    case NoSpaceLeft
    /// `EIO` — generic kernel-reported I/O failure.
    case IoFailure
    /// `EBADF` — file descriptor is invalid or closed.
    case BadFileDescriptor
    /// `EPERM` — operation not permitted.
    case NotPermitted
    /// Any other POSIX errno — keeps the original code so callers can
    /// still dispatch on the raw value.
    case Other(Int32)

    /// Classifies a POSIX errno. Unknown codes fall through to `.Other(c)`.
    public static func fromErrno(code: Int32) -> IoErrorKind {
        let code64 = Int64(from: code);
        match code64 {
            1  => .NotPermitted,
            2  => .NotFound,
            4  => .Interrupted,
            5  => .IoFailure,
            9  => .BadFileDescriptor,
            11 => .WouldBlock,
            12 => .OutOfMemory,
            13 => .PermissionDenied,
            17 => .AlreadyExists,
            20 => .NotADirectory,
            21 => .IsADirectory,
            22 => .InvalidInput,
            28 => .NoSpaceLeft,
            32 => .BrokenPipe,
            _  => .Other(code)
        }
    }

    /// The POSIX errno corresponding to this kind. Lossless round-trip
    /// for all named variants and `.Other`.
    public func errno() -> Int32 {
        match self {
            .NotPermitted      => 1,
            .NotFound          => 2,
            .Interrupted       => 4,
            .IoFailure         => 5,
            .BadFileDescriptor => 9,
            .WouldBlock        => 11,
            .OutOfMemory       => 12,
            .PermissionDenied  => 13,
            .AlreadyExists     => 17,
            .NotADirectory     => 20,
            .IsADirectory      => 21,
            .InvalidInput      => 22,
            .NoSpaceLeft       => 28,
            .BrokenPipe        => 32,
            .Other(c)          => c
        }
    }

    /// Short human-readable phrase, locale-independent.
    public func description() -> String {
        match self {
            .NotPermitted      => "operation not permitted",
            .NotFound          => "no such file or directory",
            .Interrupted       => "interrupted",
            .IoFailure         => "i/o error",
            .BadFileDescriptor => "bad file descriptor",
            .WouldBlock        => "would block",
            .OutOfMemory       => "out of memory",
            .PermissionDenied  => "permission denied",
            .AlreadyExists     => "file exists",
            .NotADirectory     => "not a directory",
            .IsADirectory      => "is a directory",
            .InvalidInput      => "invalid argument",
            .NoSpaceLeft       => "no space left",
            .BrokenPipe        => "broken pipe",
            .Other(_)          => "unknown error"
        }
    }
}

// ============================================================================
// I/O ERROR
// ============================================================================

/// Structured I/O error: a classified `kind` plus the originating POSIX
/// errno.
///
/// Returned in the `Err` arm of every `Result` produced by `Read`, `Write`,
/// `File`, `stdio`, and the `os.fs` helpers. Pattern-match on `kind` for
/// programmatic dispatch; call `description()` for a short human-readable
/// phrase. The convenience constructors at the bottom (`notFound()`,
/// `permissionDenied()`, etc.) build common kinds without spelling the enum.
///
/// # Examples
///
/// ```
/// match File.open("missing.txt") {
///     .Ok(f) => use(f),
///     .Err(e) => match e.kind {
///         .NotFound          => createDefault(),
///         .PermissionDenied  => requestAccess(),
///         _                  => log(e.description())
///     }
/// }
/// ```
public struct IoError {
    public var kind: IoErrorKind

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name From Kind
    /// Builds an error for a categorized kind.
    public init(kind kind: IoErrorKind) {
        self.kind = kind;
    }

    /// @name From Code
    /// Builds an error from a raw POSIX errno; classifies the kind.
    public init(code code: Int32) {
        self.kind = IoErrorKind.fromErrno(code);
    }

    /// Snapshots the current value of the platform's `errno` thread-local.
    /// Call immediately after a failed libc call — any other libc activity
    /// in between can clobber the value.
    public static func last() -> IoError {
        IoError(code: libc.errno())
    }

    // ========================================================================
    // ERROR INFORMATION
    // ========================================================================

    /// Returns a short human-readable phrase for the error kind. Unknown
    /// codes yield `"unknown error"`; for full coverage use `errno()`
    /// with a platform `strerror`.
    public func description() -> String {
        self.kind.description()
    }

    /// The raw POSIX error code. Use for programmatic dispatch when
    /// pattern-matching on `kind` is too coarse — e.g. distinguishing
    /// between `.Other` codes.
    public func errno() -> Int32 {
        self.kind.errno()
    }
}

// ============================================================================
// COMMON ERROR CONSTRUCTORS
// ============================================================================
//
// Convenience shorthands for the most-used kinds. Equivalent to
// `IoError(kind: .NotFound)` etc., but read more naturally at error sites.

/// `ENOENT` — the path does not exist.
public func notFound() -> IoError { IoError(kind: .NotFound) }

/// `EACCES` — caller lacks permission for the operation.
public func permissionDenied() -> IoError { IoError(kind: .PermissionDenied) }

/// `EEXIST` — the path already exists (e.g. `O_CREAT | O_EXCL`).
public func alreadyExists() -> IoError { IoError(kind: .AlreadyExists) }

/// `EINVAL` — invalid argument to a libc call.
public func invalidInput() -> IoError { IoError(kind: .InvalidInput) }

/// `EAGAIN` — non-blocking call would have blocked.
public func wouldBlock() -> IoError { IoError(kind: .WouldBlock) }

/// `EINTR` — operation interrupted by a signal.
public func interrupted() -> IoError { IoError(kind: .Interrupted) }

/// `EPIPE` — write to a pipe with no reader.
public func brokenPipe() -> IoError { IoError(kind: .BrokenPipe) }
