/// Error types for Swoop HTTP requests.

module swoop.error

/// The kind of error that occurred during an HTTP request.
public enum SwoopErrorKind: Cloneable {
    /// Failed to connect to the remote host.
    case ConnectionFailed(String)

    /// The request timed out.
    case Timeout

    /// The URL could not be parsed.
    case InvalidUrl(String)

    /// The server returned a non-2xx status (from validate()).
    /// Contains the status code.
    case HttpError(Int64)

    /// Failed to parse the HTTP response from the server.
    case InvalidResponse(String)

    public func clone() -> SwoopErrorKind {
        match self {
            .ConnectionFailed(msg) => SwoopErrorKind.ConnectionFailed(msg.clone()),
            .Timeout => SwoopErrorKind.Timeout,
            .InvalidUrl(msg) => SwoopErrorKind.InvalidUrl(msg.clone()),
            .HttpError(code) => SwoopErrorKind.HttpError(code),
            .InvalidResponse(msg) => SwoopErrorKind.InvalidResponse(msg.clone())
        }
    }
}

/// An error that occurred during an HTTP request.
public struct SwoopError: Cloneable {
    public var kind: SwoopErrorKind

    public init(kind: SwoopErrorKind) {
        self.kind = kind;
    }

    public func clone() -> SwoopError {
        SwoopError(self.kind.clone())
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        match self.kind {
            .ConnectionFailed(msg) => "connection failed: " + msg,
            .Timeout => "request timed out",
            .InvalidUrl(msg) => "invalid URL: " + msg,
            .HttpError(code) => "HTTP error: status " + code.format(),
            .InvalidResponse(msg) => "invalid response: " + msg
        }
    }

    public static func connectionFailed(message: String) -> SwoopError = SwoopError(SwoopErrorKind.ConnectionFailed(message))
    public static func timeout() -> SwoopError = SwoopError(SwoopErrorKind.Timeout)
    public static func invalidUrl(message: String) -> SwoopError = SwoopError(SwoopErrorKind.InvalidUrl(message))
    public static func httpError(code: Int64) -> SwoopError = SwoopError(SwoopErrorKind.HttpError(code))
    public static func invalidResponse(message: String) -> SwoopError = SwoopError(SwoopErrorKind.InvalidResponse(message))
}
