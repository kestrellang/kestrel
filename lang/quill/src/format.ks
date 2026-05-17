/// Pluggable serialization format protocol.
///
/// `Format` decouples encoding/decoding logic from `Value` trees so
/// the same `Serialize`/`Deserialize` conformances work across JSON,
/// TOML, or any future wire format. Libraries like Swoop use
/// `contentType()` for automatic content negotiation.
///
/// # Examples
///
/// ```
/// // Given a type `Json` conforming to `Format`:
/// let v = Value.Obj(["key": Value.Str("val")]);
/// let json = try Json.encode(v);      // Ok("{\"key\":\"val\"}")
/// let back = try Json.decode(json);   // Ok(.Obj({"key": .Str("val")}))
/// Json.contentType();                 // "application/json"
/// ```

module quill.format

import quill.value.(Value)
import quill.error.(SerializeError, DeserializeError)

// ============================================================================
// FORMAT PROTOCOL
// ============================================================================

/// A serialization format that can encode `Value` trees to strings and
/// decode strings back into `Value` trees.
///
/// Conforming types (e.g. `Json`, `Toml`) provide a stateless,
/// static-only API. Each format is responsible for its own escaping,
/// whitespace, and syntax rules; the `Value` layer handles only the
/// abstract data model.
///
/// # Examples
///
/// ```
/// // Encode with a specific format:
/// let s = try Json.encode(myValue);
///
/// // Decode from a string:
/// let v = try Json.decode(jsonString);
///
/// // Content negotiation:
/// let mime = Json.contentType();  // "application/json"
/// ```
public protocol Format {
    /// Encodes a `Value` tree to a formatted string.
    ///
    /// Returns the serialized string on success, or a
    /// `SerializeError` if the value contains data the format
    /// cannot represent.
    static func encode(value: Value) -> Result[String, SerializeError]

    /// Decodes a formatted string back into a `Value` tree.
    ///
    /// Returns the parsed `Value` on success, or a
    /// `DeserializeError` if the input is malformed or violates
    /// the format's grammar.
    static func decode(source: String) -> Result[Value, DeserializeError]

    /// Returns the MIME content type for this format
    /// (e.g. `"application/json"`, `"application/toml"`).
    static func contentType() -> String
}
