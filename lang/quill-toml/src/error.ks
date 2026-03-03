// TOML parse error type

module quill.toml.error


// ============================================================================
// TOML PARSE ERROR
// ============================================================================

/// An error encountered while parsing TOML.
public struct TomlParseError: Cloneable {
    public var message: String
    public var line: Int64

    public init(message: String, line: Int64) {
        self.message = message;
        self.line = line;
    }

    public func clone() -> TomlParseError {
        TomlParseError(self.message.clone(), self.line)
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        "TOML parse error at line " + self.line.format() + ": " + self.message
    }
}
