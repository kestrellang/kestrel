/// TOML serialization format for the quill framework.
///
/// This module provides `Toml`, a `Format` conformer that encodes and
/// decodes quill `Value` trees as TOML text, plus the convenience
/// function `toToml` that combines serialization and encoding in one call.
///
/// The parser supports the subset of TOML needed for `flock.toml` config
/// files: bare keys, basic quoted strings, integers, floats, booleans,
/// arrays, inline tables, and standard `[section]` tables. Datetime,
/// array-of-tables `[[...]]`, multiline strings, dotted keys, and literal
/// strings are not supported.
///
/// # Examples
///
/// ```
/// import quill.toml.(Toml, toToml)
/// import quill.format.(Format)
///
/// let v = Value.Obj([("name", Value.Str("my-pkg")), ("version", Value.Str("0.1.0"))]);
/// let encoded = try Toml.encode(value: v);
/// // encoded == "name = \"my-pkg\"\nversion = \"0.1.0\"\n"
///
/// let decoded = try Toml.decode(source: encoded);
/// // decoded == v
/// ```

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

/// TOML serialization format for use with the quill `Format` protocol.
///
/// `Toml` is a stateless type whose static methods convert between `Value`
/// and TOML text. The top-level value must be a `.Obj` — TOML documents
/// are always key-value tables at the root.
///
/// Encoding can fail if the root value is not an object. Decoding can fail
/// on syntax errors; the error carries a line number.
///
/// # Examples
///
/// ```
/// let v = Value.Obj([("debug", Value.Boolean(true))]);
/// let toml = try Toml.encode(value: v);  // "debug = true\n"
/// let back = try Toml.decode(source: toml);  // v
/// ```
public struct Toml: Format {
    /// Encodes a `Value` to a TOML string.
    ///
    /// The value must be `.Obj` at the top level (TOML requires a root table).
    /// Returns a `SerializeError` if the root is any other variant.
    public static func encode(value: Value) -> Result[String, SerializeError] {
        emitToml(value)
    }

    /// Decodes a TOML string into a `Value`.
    ///
    /// Returns a `DeserializeError` if the input contains unsupported TOML
    /// features or syntax violations. The underlying `TomlParseError` carries
    /// the line number; that detail is folded into the error description.
    public static func decode(source: String) -> Result[Value, DeserializeError] {
        match parseToml(source) {
            .Ok(v) => .Ok(v),
            .Err(e) => .Err(DeserializeError.custom(e.description()))
        }
    }

    /// Returns the MIME content type for TOML: `"application/toml"`.
    public static func contentType() -> String { "application/toml" }
}

// ============================================================================
// CONVENIENCE API
// ============================================================================

/// Serializes any `Serialize` value to a TOML string.
///
/// Combines `value.toValue()` and `emitToml` in a single call. The
/// serialized `Value` must be a root `.Obj`; other shapes produce an error.
///
/// # Examples
///
/// ```
/// let toml = try toToml(value: myConfig);
/// ```
public func toToml[T](value: T) -> Result[String, SerializeError] where T: Serialize {
    let v = try value.toValue();
    emitToml(v)
}
