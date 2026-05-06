module clutch.error

/// Describes a failure that occurred while parsing command-line arguments.
///
/// Returned as the `.Err` variant of the `Result` from
/// `Command.parse(from:)`. Each case carries a `String` payload with
/// context — typically the flag or argument name that caused the error.
/// Call `description()` to get a human-readable `"error: ..."` message
/// suitable for printing to stderr.
///
/// The `.Message` case is special: it carries pre-formatted text (used
/// by the built-in `--help` / `-h` handler to surface help output
/// through the same error path).
///
/// # Examples
///
/// ```
/// let err = ParseError.UnknownFlag("--foo");
/// err.description();  // "error: unknown flag: --foo"
///
/// let help = ParseError.Message("mycli 1.0\n...");
/// help.description();  // "mycli 1.0\n..."
/// ```
public enum ParseError: Cloneable {
    /// An unrecognized `--flag` or `-f` was encountered.
    /// Payload: the flag as typed (e.g., `"--foo"`).
    case UnknownFlag(String)

    /// An option flag was present but its value was missing.
    /// Payload: the flag as typed (e.g., `"--output"`).
    case MissingValue(String)

    /// A required argument was not provided and has no default.
    /// Payload: the argument name (e.g., `"file"`).
    case MissingRequired(String)

    /// A bare token did not match any registered subcommand.
    /// Payload: the token that was entered.
    case UnknownSubcommand(String)

    /// A positional value appeared but no positional argument
    /// definition exists at that index.
    /// Payload: the unexpected value.
    case UnexpectedPositional(String)

    /// A pre-formatted message, typically auto-generated help text.
    /// `description()` returns the payload as-is.
    case Message(String)
}

extend ParseError {
    /// Creates a deep copy of the error, cloning the payload string.
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

    /// Formats the error as a human-readable string.
    ///
    /// Every case except `.Message` produces an `"error: ..."` prefix
    /// followed by a description and the payload. `.Message` returns its
    /// payload verbatim (used for help-text passthrough).
    ///
    /// # Examples
    ///
    /// ```
    /// ParseError.MissingRequired("file").description();
    /// // "error: missing required argument: file"
    ///
    /// ParseError.UnknownFlag("--foo").description();
    /// // "error: unknown flag: --foo"
    /// ```
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
