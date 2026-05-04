/// JSON serialization format for the quill framework.
///
/// This module provides `Json`, a `Format` conformer that encodes and
/// decodes quill `Value` trees as JSON text, plus the convenience
/// functions `toJson` and `toJsonPretty` that combine serialization and
/// encoding in a single call.
///
/// # Examples
///
/// ```
/// import quill.json.(Json, toJson, toJsonPretty)
/// import quill.format.(Format)
///
/// // Round-trip through the Format protocol
/// let encoded = try Json.encode(value: Value.Int(42));  // "42"
/// let decoded = try Json.decode(source: encoded);       // Value.Int(42)
///
/// // Convenience API for Serialize types
/// let compact = try toJson(value: myStruct);
/// let pretty  = try toJsonPretty(value: myStruct);
/// ```

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

/// JSON serialization format for use with the quill `Format` protocol.
///
/// `Json` is a stateless type whose static methods convert between `Value`
/// and JSON text. Pass it as a type argument to any generic function that
/// accepts a `Format`, or call its methods directly for one-off encoding
/// and decoding.
///
/// Encoding always succeeds (every `Value` has a JSON representation).
/// Decoding can fail if the input is malformed; the error carries a byte
/// offset pointing at the problem.
///
/// # Examples
///
/// ```
/// let v = Value.Obj([("name", Value.Str("Alice")), ("age", Value.Int(30))]);
/// let json = try Json.encode(value: v);
/// // json == "{\"name\":\"Alice\",\"age\":30}"
///
/// let back = try Json.decode(source: json);
/// // back == v
/// ```
public struct Json: Format {
    /// Encodes a `Value` to a compact JSON string with no extra whitespace.
    ///
    /// This always succeeds — every well-formed `Value` maps to valid JSON.
    /// The output uses no indentation or trailing newlines; use `toJsonPretty`
    /// when human readability matters.
    public static func encode(value: Value) -> Result[String, SerializeError] {
        .Ok(emitJson(value))
    }

    /// Decodes a JSON string into a `Value`.
    ///
    /// Returns a `DeserializeError` if the input is not syntactically valid
    /// JSON. The underlying `JsonParseError` carries the byte offset of the
    /// first problem; that detail is folded into the error description.
    ///
    /// # Errors
    ///
    /// Returns `.Err` for any JSON syntax violation — unterminated strings,
    /// trailing commas, unquoted keys, bare identifiers other than `true`,
    /// `false`, or `null`.
    public static func decode(source: String) -> Result[Value, DeserializeError] {
        match parseJson(source) {
            .Ok(v) => .Ok(v),
            .Err(e) => .Err(DeserializeError.custom(e.description()))
        }
    }

    /// Returns the MIME content type for JSON: `"application/json"`.
    public static func contentType() -> String { "application/json" }
}

// ============================================================================
// CONVENIENCE API
// ============================================================================

/// Serializes any `Serialize` value to a compact JSON string.
///
/// Combines `value.toValue()` and `emitJson` in a single call. Fails only
/// if the `Serialize` implementation itself returns an error.
///
/// # Examples
///
/// ```
/// let json = try toJson(value: myUser);
/// // json == "{\"name\":\"Alice\",\"age\":30}"
/// ```
public func toJson[T](value: T) -> Result[String, SerializeError] where T: Serialize {
    let v = try value.toValue();
    .Ok(emitJson(v))
}

/// Serializes any `Serialize` value to a pretty-printed JSON string.
///
/// Identical to `toJson` except the output uses 2-space indentation and
/// newlines between elements for human readability.
///
/// # Examples
///
/// ```
/// let json = try toJsonPretty(value: myUser);
/// // json == "{\n  \"name\": \"Alice\",\n  \"age\": 30\n}"
/// ```
public func toJsonPretty[T](value: T) -> Result[String, SerializeError] where T: Serialize {
    let v = try value.toValue();
    .Ok(emitJsonPretty(v))
}
