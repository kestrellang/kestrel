// JSON parse error type

module quill.json.error


// ============================================================================
// JSON PARSE ERROR
// ============================================================================

/// An error encountered while parsing JSON.
public struct JsonParseError: Cloneable {
    public var message: String
    public var offset: Int64

    public init(message: String, offset: Int64) {
        self.message = message;
        self.offset = offset;
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        "JSON parse error at offset " + self.offset.format() + ": " + self.message
    }

    public func clone() -> JsonParseError {
        JsonParseError(self.message.clone(), self.offset)
    }
}
