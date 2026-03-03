// Quill Format protocol for pluggable serialization formats

module quill.format

import quill.value.(Value)
import quill.error.(SerializeError, DeserializeError)

// ============================================================================
// FORMAT PROTOCOL
// ============================================================================

/// A serialization format that can encode Values to strings and decode strings
/// back to Values. Conforming types (e.g. Json, Toml) can be used generically
/// with libraries like Swoop for automatic content negotiation.
public protocol Format {
    /// Encodes a Value to a formatted string (e.g. JSON, TOML).
    static func encode(value: Value) -> Result[String, SerializeError]

    /// Decodes a formatted string back into a Value.
    static func decode(source: String) -> Result[Value, DeserializeError]

    /// Returns the MIME content type for this format (e.g. "application/json").
    static func contentType() -> String
}
