/// Error type for SQLite operations.

module talon.sqlite.error

public enum SqliteError: Cloneable {
    case Error(String)

    public func clone() -> SqliteError {
        match self {
            .Error(msg) => SqliteError.Error(msg.clone())
        }
    }

    public func description() -> String {
        match self {
            .Error(msg) => "sqlite error: " + msg
        }
    }
}
