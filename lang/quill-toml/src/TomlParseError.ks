/// Error type returned when TOML parsing fails.

module quill.toml.error


// ============================================================================
// TOML PARSE ERROR
// ============================================================================

/// A syntax error encountered while parsing a TOML string.
///
/// Carries a human-readable `message` describing what went wrong and the
/// 1-based `line` number where the parser stopped. Use `description()` to
/// format both into a single diagnostic string.
///
/// # Representation
///
/// Two fields: `message` (a short explanation like "expected '=' in
/// key-value pair") and `line` (the 1-based source line number).
///
/// # Examples
///
/// ```
/// let err = TomlParseError("unterminated table header", 5);
/// err.description();  // "TOML parse error at line 5: unterminated table header"
/// ```
public struct TomlParseError: Cloneable {
    /// Short explanation of the syntax violation.
    public var message: String

    /// 1-based line number in the source where parsing failed.
    public var line: Int64

    /// @name Default
    /// Creates an error with the given message and line number.
    public init(message: String, line: Int64) {
        self.message = message;
        self.line = line;
    }

    /// Returns a deep copy of this error.
    public func clone() -> TomlParseError {
        TomlParseError(self.message.clone(), self.line)
    }

    /// Formats the error as `"TOML parse error at line <N>: <message>"`.
    public func description() -> String {
        "TOML parse error at line " + self.line.format() + ": " + self.message
    }
}
