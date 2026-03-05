// Clutch parse error types

module clutch.error

/// An error that occurred during argument parsing.
public enum ParseError: Cloneable {
    /// An unknown flag was provided.
    case UnknownFlag(String)

    /// An option was missing its value.
    case MissingValue(String)

    /// A required argument was not provided.
    case MissingRequired(String)

    /// An unknown subcommand was provided.
    case UnknownSubcommand(String)

    /// A positional argument appeared where none was expected.
    case UnexpectedPositional(String)

    /// A general parsing message (used for help text).
    case Message(String)
}

extend ParseError {
    public func clone() -> ParseError {
        match self {
            .UnknownFlag(s) => .UnknownFlag(s.clone()),
            .MissingValue(s) => .MissingValue(s.clone()),
            .MissingRequired(s) => .MissingRequired(s.clone()),
            .UnknownSubcommand(s) => .UnknownSubcommand(s.clone()),
            .UnexpectedPositional(s) => .UnexpectedPositional(s.clone()),
            .Message(s) => .Message(s.clone())
        }
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        match self {
            .UnknownFlag(name) => {
                var msg = String();
                msg.append("error: unknown flag: ");
                msg.append(name);
                msg
            },
            .MissingValue(name) => {
                var msg = String();
                msg.append("error: missing value for option: ");
                msg.append(name);
                msg
            },
            .MissingRequired(name) => {
                var msg = String();
                msg.append("error: missing required argument: ");
                msg.append(name);
                msg
            },
            .UnknownSubcommand(name) => {
                var msg = String();
                msg.append("error: unknown subcommand: ");
                msg.append(name);
                msg
            },
            .UnexpectedPositional(val) => {
                var msg = String();
                msg.append("error: unexpected positional argument: ");
                msg.append(val);
                msg
            },
            .Message(msg) => msg
        }
    }
}
