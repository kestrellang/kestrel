// IoError and IoErrorKind: what went wrong in an I/O call.

module std.streams

import std.numeric.(Int32, Int64)
import std.text.(String, Formatter, Formattable, FormatOptions, _writePadded)
import std.core.(Bool, Equatable)
import std.result.(Optional)
import std.memory.(RawPointer)
import std.ffi.(CString)

// ============================================================================
// KIND
// ============================================================================

/// What kind of failure an `IoError` is. Match on it to handle the cases
/// that matter; everything unclassified is `.Other`, and `IoError.errno`
/// still has the system's code.
///
/// # Examples
///
/// ```
/// match readText("config.toml") {
///     .Ok(text) => parse(text),
///     .Err(e) if e.kind == .NotFound => defaults(),
///     .Err(e) => fail(e)
/// }
/// ```
public enum IoErrorKind: Equatable, Formattable {
    /// Not one of the kinds below; see `IoError.errno`.
    case Other
    /// The file or directory doesn't exist.
    case NotFound
    /// The process lacks permission.
    case PermissionDenied
    /// The path already exists.
    case AlreadyExists
    /// An argument was invalid.
    case InvalidInput
    /// The bytes weren't in the expected format, such as UTF-8.
    case InvalidData
    /// The stream ended before the expected number of bytes.
    case UnexpectedEnd
    /// The operation ran out of time.
    case TimedOut
    /// A non-blocking call would have had to wait.
    case WouldBlock
    /// A signal interrupted the call; it can be retried.
    case Interrupted
    /// The other end of a pipe or socket was closed.
    case BrokenPipe
    /// Memory ran out.
    case OutOfMemory
    /// A path component isn't a directory.
    case NotADirectory
    /// A file was expected, but the path is a directory.
    case IsADirectory
    /// The device is full.
    case NoSpaceLeft
    /// The device reported a low-level I/O failure.
    case IoFailure
    /// The descriptor is closed or invalid.
    case BadFileDescriptor
    /// The operation isn't permitted.
    case NotPermitted
    /// The remote host refused the connection.
    case ConnectionRefused
    /// The remote host reset the connection.
    case ConnectionReset
    /// The connection was aborted on this side.
    case ConnectionAborted
    /// The socket isn't connected.
    case NotConnected
    /// The address is already in use.
    case AddressInUse
    /// The address isn't available on this machine.
    case AddressNotAvailable
    /// The host name couldn't be resolved.
    case HostNotFound
    /// The directory isn't empty.
    case DirectoryNotEmpty
    /// A rename or link would cross file systems.
    case CrossesDevices
    /// The system doesn't support the operation.
    case Unsupported

    /// The kind for a system error code.
    public static func of(errno code: Int32) -> IoErrorKind {
        match Int64(from: native_errno_kind(code)) {
            1 => .NotFound, 2 => .PermissionDenied, 3 => .AlreadyExists,
            4 => .InvalidInput, 5 => .InvalidData, 6 => .UnexpectedEnd,
            7 => .TimedOut, 8 => .WouldBlock, 9 => .Interrupted,
            10 => .BrokenPipe, 11 => .OutOfMemory, 12 => .NotADirectory,
            13 => .IsADirectory, 14 => .NoSpaceLeft, 15 => .IoFailure,
            16 => .BadFileDescriptor, 17 => .NotPermitted,
            18 => .ConnectionRefused, 19 => .ConnectionReset,
            20 => .ConnectionAborted, 21 => .NotConnected,
            22 => .AddressInUse, 23 => .AddressNotAvailable,
            24 => .HostNotFound, 25 => .DirectoryNotEmpty,
            26 => .CrossesDevices, 27 => .Unsupported,
            _ => .Other
        }
    }

    /// A short lowercase phrase, the same on every platform.
    public func description() -> String {
        match self {
            .Other => "other error",
            .NotFound => "not found",
            .PermissionDenied => "permission denied",
            .AlreadyExists => "already exists",
            .InvalidInput => "invalid input",
            .InvalidData => "invalid data",
            .UnexpectedEnd => "unexpected end of stream",
            .TimedOut => "timed out",
            .WouldBlock => "operation would block",
            .Interrupted => "interrupted",
            .BrokenPipe => "broken pipe",
            .OutOfMemory => "out of memory",
            .NotADirectory => "not a directory",
            .IsADirectory => "is a directory",
            .NoSpaceLeft => "no space left on device",
            .IoFailure => "i/o failure",
            .BadFileDescriptor => "bad file descriptor",
            .NotPermitted => "operation not permitted",
            .ConnectionRefused => "connection refused",
            .ConnectionReset => "connection reset",
            .ConnectionAborted => "connection aborted",
            .NotConnected => "not connected",
            .AddressInUse => "address in use",
            .AddressNotAvailable => "address not available",
            .HostNotFound => "host not found",
            .DirectoryNotEmpty => "directory not empty",
            .CrossesDevices => "crosses devices",
            .Unsupported => "unsupported"
        }
    }

    /// Same kind.
    public func isEqual(to other: IoErrorKind) -> Bool {
        self.description() == other.description()
    }

    /// Writes `description()`.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        _writePadded(into: writer, self.description(), options)
    }
}

// ============================================================================
// ERROR
// ============================================================================

/// A failed I/O call: its `kind`, plus the system error code when the
/// system reported one.
///
/// # Examples
///
/// ```
/// match File.open("data.bin") {
///     .Ok(file) => use(file),
///     .Err(e) => eprintln("can't open data.bin: \(e)")
/// }
/// ```
public struct IoError: Equatable, Formattable {
    /// What kind of failure this is.
    public var kind: IoErrorKind
    /// The system error code, or -1 for an error std made itself.
    var code: Int32

    /// @name From Kind
    /// An error of `kind` with no system error code.
    public init(kind kind: IoErrorKind) {
        self.kind = kind;
        self.code = -1;
    }

    /// @name From Errno
    /// The error for a system error code.
    public init(errno code: Int32) {
        self.kind = IoErrorKind.of(errno: code);
        self.code = code;
    }

    /// The error for the calling thread's current `errno`. Call it right
    /// after the failing system call, before anything else can change it.
    public static func last() -> IoError {
        IoError(errno: native_errno())
    }

    /// The system error code, if the system reported this error.
    public var errno: Optional[Int32] {
        if self.code < 0 { .None } else { .Some(self.code) }
    }

    /// A short message: the kind's description, or the system's message
    /// for an error of kind `.Other`.
    public func description() -> String {
        if self.code >= 0 and self.kind == .Other {
            let text = String(from: CString(raw: native_strerror(self.code).cast[UInt8]()));
            return "\(text) (errno \(self.code))"
        }
        self.kind.description()
    }

    /// Same kind and same system error code.
    public func isEqual(to other: IoError) -> Bool {
        self.kind == other.kind and self.code == other.code
    }

    /// Writes `description()`.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        _writePadded(into: writer, self.description(), options)
    }

    /// `NotFound`.
    public static var notFound: IoError { IoError(kind: .NotFound) }
    /// `PermissionDenied`.
    public static var permissionDenied: IoError { IoError(kind: .PermissionDenied) }
    /// `AlreadyExists`.
    public static var alreadyExists: IoError { IoError(kind: .AlreadyExists) }
    /// `InvalidInput`.
    public static var invalidInput: IoError { IoError(kind: .InvalidInput) }
    /// `InvalidData`.
    public static var invalidData: IoError { IoError(kind: .InvalidData) }
    /// `UnexpectedEnd`.
    public static var unexpectedEnd: IoError { IoError(kind: .UnexpectedEnd) }
    /// `TimedOut`.
    public static var timedOut: IoError { IoError(kind: .TimedOut) }
    /// `WouldBlock`.
    public static var wouldBlock: IoError { IoError(kind: .WouldBlock) }
    /// `Interrupted`.
    public static var interrupted: IoError { IoError(kind: .Interrupted) }
    /// `BrokenPipe`.
    public static var brokenPipe: IoError { IoError(kind: .BrokenPipe) }
    /// `Unsupported`.
    public static var unsupported: IoError { IoError(kind: .Unsupported) }
}

@extern(.C, mangleName: "kestrel_errno")
internal func native_errno() -> Int32

@extern(.C, mangleName: "kestrel_errno_kind")
internal func native_errno_kind(code: Int32) -> Int32

@extern(.C, mangleName: "kestrel_strerror")
internal func native_strerror(code: Int32) -> RawPointer
