// Jessup error types

module jessup.error

/// All error conditions that can occur in jessup operations.
public enum JessupError: Cloneable {
    case ConfigError(String)
    case NetworkError(String)
    case InstallError(String)
    case NotFound(String)
    case IoError(String)
    case ParseError(String)

    public func clone() -> JessupError {
        match self {
            .ConfigError(s) => .ConfigError(s.clone()),
            .NetworkError(s) => .NetworkError(s.clone()),
            .InstallError(s) => .InstallError(s.clone()),
            .NotFound(s) => .NotFound(s.clone()),
            .IoError(s) => .IoError(s.clone()),
            .ParseError(s) => .ParseError(s.clone())
        }
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        match self {
            .ConfigError(msg) => "config error: " + msg,
            .NetworkError(msg) => "network error: " + msg,
            .InstallError(msg) => "install error: " + msg,
            .NotFound(msg) => "not found: " + msg,
            .IoError(msg) => "I/O error: " + msg,
            .ParseError(msg) => "parse error: " + msg
        }
    }
}
