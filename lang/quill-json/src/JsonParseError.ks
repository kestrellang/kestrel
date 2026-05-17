/// Error type returned when JSON parsing fails.

module quill.json.error


// ============================================================================
// JSON PARSE ERROR
// ============================================================================

/// A syntax error encountered while parsing a JSON string.
///
/// Carries a human-readable `message` describing what went wrong and the
/// byte `offset` into the source where the parser stopped. Use
/// `description()` to format both into a single diagnostic string.
///
/// # Representation
///
/// Two fields: `message` (a short explanation like "unexpected character '}'")
/// and `offset` (the zero-based byte position in the source string).
///
/// # Examples
///
/// ```
/// let err = JsonParseError("unexpected end of input", 42);
/// err.description();  // "JSON parse error at offset 42: unexpected end of input"
/// ```
public struct JsonParseError: Cloneable {
    /// Short explanation of the syntax violation.
    public var message: String

    /// Zero-based byte offset into the source where parsing failed.
    public var offset: Int64

    /// @name Default
    /// Creates an error with the given message and byte offset.
    public init(message: String, offset: Int64) {
        self.message = message;
        self.offset = offset;
    }

    /// Formats the error as `"JSON parse error at offset <N>: <message>"`.
    public func description() -> String {
        "JSON parse error at offset \(self.offset): \(self.message)"
    }

    /// Returns a deep copy of this error.
    public func clone() -> JsonParseError {
        JsonParseError(self.message.clone(), self.offset)
    }
}
