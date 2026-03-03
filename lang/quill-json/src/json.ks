// JSON convenience API - combines parsing/emitting with serialize/deserialize

module quill.json

import quill.value.(Value)
import quill.error.(SerializeError, DeserializeError)
import quill.serialize.(Serialize)
import quill.deserialize.(Deserialize)
import quill.format.(Format)
import quill.json.error.(JsonParseError)
import quill.json.parser.(parseJson)
import quill.json.emitter.(emitJson, emitJsonPretty)

// ============================================================================
// FORMAT CONFORMANCE
// ============================================================================

/// JSON serialization format for use with the Format protocol.
public struct Json: Format {
    /// Encodes a Value to a compact JSON string.
    public static func encode(value: Value) -> Result[String, SerializeError] {
        .Ok(emitJson(value))
    }

    /// Decodes a JSON string into a Value.
    public static func decode(source: String) -> Result[Value, DeserializeError] {
        match parseJson(source) {
            .Ok(v) => .Ok(v),
            .Err(e) => .Err(DeserializeError.custom(e.description()))
        }
    }

    /// Returns the MIME content type for JSON.
    public static func contentType() -> String { "application/json" }
}

// ============================================================================
// CONVENIENCE API
// ============================================================================

/// Serializes a value to a compact JSON string.
public func toJson[T](value: T) -> Result[String, SerializeError] where T: Serialize {
    let v = try value.toValue();
    .Ok(emitJson(v))
}

/// Serializes a value to a pretty-printed JSON string.
public func toJsonPretty[T](value: T) -> Result[String, SerializeError] where T: Serialize {
    let v = try value.toValue();
    .Ok(emitJsonPretty(v))
}
