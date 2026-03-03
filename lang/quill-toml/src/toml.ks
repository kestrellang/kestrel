// TOML convenience API - combines parsing/emitting with serialize/deserialize

module quill.toml

import quill.value.(Value)
import quill.error.(SerializeError, DeserializeError)
import quill.serialize.(Serialize)
import quill.deserialize.(Deserialize)
import quill.format.(Format)
import quill.toml.error.(TomlParseError)
import quill.toml.parser.(parseToml)
import quill.toml.emitter.(emitToml)

// ============================================================================
// FORMAT CONFORMANCE
// ============================================================================

/// TOML serialization format for use with the Format protocol.
public struct Toml: Format {
    /// Encodes a Value to a TOML string.
    public static func encode(value: Value) -> Result[String, SerializeError] {
        emitToml(value)
    }

    /// Decodes a TOML string into a Value.
    public static func decode(source: String) -> Result[Value, DeserializeError] {
        match parseToml(source) {
            .Ok(v) => .Ok(v),
            .Err(e) => .Err(DeserializeError.custom(e.description()))
        }
    }

    /// Returns the MIME content type for TOML.
    public static func contentType() -> String { "application/toml" }
}

// ============================================================================
// CONVENIENCE API
// ============================================================================

/// Serializes a value to a TOML string.
public func toToml[T](value: T) -> Result[String, SerializeError] where T: Serialize {
    let v = try value.toValue();
    emitToml(v)
}
