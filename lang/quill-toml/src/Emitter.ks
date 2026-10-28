/// Converts a quill `Value` tree into a TOML string.
///
/// The top-level value must be `.Obj` (TOML documents are always root
/// tables). Sub-tables are emitted with `[section]` headers; non-table
/// values are emitted as `key = value` lines.

module quill.toml.emitter

import quill.value.(Value)
import quill.error.(SerializeError, SerializeErrorKind)
import quill.toml.parser.(containsFloatMarker)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Converts a `Value` to a TOML string.
///
/// The root must be `.Obj`; any other variant produces a `SerializeError`.
/// Sub-objects become `[section]` tables; nested objects within those become
/// dotted section headers (e.g., `[package.dependencies]`).
///
/// # Examples
///
/// ```
/// let v = Value.Obj([("name", Value.Str("hello")), ("version", Value.Str("1.0"))]);
/// let s = try emitToml(v);  // "name = \"hello\"\nversion = \"1.0\"\n"
/// ```
///
/// # Errors
///
/// Returns `.Err` if the root value is not `.Obj`.
public func emitToml(value: Value) -> String throws SerializeError {
    match value {
        .Obj(obj) => {
            var buf = String();
            emitTable(obj, buf, "");
            .Ok(buf)
        },
        _ => throw SerializeError.custom("TOML top-level value must be an object")
    }
}

// ============================================================================
// TABLE EMITTER
// ============================================================================

/// Emits a table's key-value pairs, then its sub-tables with `[section]` headers.
///
/// Two-pass approach: scalar values first (so they appear before any section
/// break), then nested objects with their `[prefix.key]` headers.
func emitTable(obj: [String: Value], mutating buf: String, prefix: String) {
    // First pass: emit non-table values as key = value
    for (key, val) in obj.iter() {
        match val {
            .Obj(_) => {},  // skip tables for now
            _ => {
                emitKey(key, buf);
                buf.append(" = ");
                emitTomlValue(val, buf);
                buf.append("\n")
            }
        }
    }

    // Second pass: emit sub-tables with [section] headers
    for (key, val) in obj.iter() {
        match val {
            .Obj(subObj) => {
                let fullKey = if not prefix.isEmpty {
                    prefix + "." + key
                } else {
                    key
                };
                buf.append("\n");
                buf.append("[");
                buf.append(fullKey);
                buf.append("]");
                buf.append("\n");
                emitTable(subObj, buf, fullKey)
            },
            _ => {}  // already emitted
        }
    }
}

// ============================================================================
// VALUE EMITTER
// ============================================================================

/// Emits a single value in TOML inline form (right-hand side of `key = ...`).
func emitTomlValue(value: Value, mutating buf: String) {
    match value {
        .Null => buf.append("\"\""),  // TOML has no null; emit empty string
        .Boolean(b) => {
            if b {
                buf.append("true")
            } else {
                buf.append("false")
            }
        },
        .Int(n) => buf.append("\(n)"),
        .Float(f) => emitFloat(f, buf),
        .Str(s) => emitTomlString(s, buf),
        .Arr(arr) => {
            buf.append("[");
            for (index, item) in arr.iter().enumerate() {
                if index > 0 {
                    buf.append(", ")
                }
                emitTomlValue(item, buf)
            }
            buf.append("]")
        },
        .Obj(_) => {
            // Nested objects shouldn't appear as inline values in our emitter
            buf.append("{}")
        }
    }
}

// ============================================================================
// STRING/KEY EMITTING
// ============================================================================

/// Emits a float in TOML spelling.
///
/// Non-finite values use the `inf`/`-inf`/`nan` tokens — `"\(f)"` would render
/// them as `Infinity`/`NaN`, which no TOML reader accepts. Finite values always
/// carry a `.` or exponent so they don't re-read as integers.
func emitFloat(f: Float64, mutating buf: String) {
    if f.isNaN {
        buf.append("nan");
        return;
    }

    if f.isInfinite {
        let token = if f < 0.0 { "-inf" } else { "inf" };
        buf.append(token);
        return;
    }

    let rendered = "\(f)";
    buf.append(rendered);
    if not containsFloatMarker(rendered) {
        buf.append(".0")
    }
}

/// Emits a TOML key — bare if it contains only `[A-Za-z0-9_-]`, quoted otherwise.
func emitKey(key: String, mutating buf: String) {
    if isBareKey(key) {
        buf.append(key)
    } else {
        emitTomlString(key, buf)
    }
}

/// Returns `true` if the string is a valid bare TOML key (`[A-Za-z0-9_-]+`).
func isBareKey(s: String) -> Bool {
    if s.isEmpty {
        return false
    }
    for c in s {
        if not (c.isAsciiLetter or c.isAsciiDigit or c == '-' or c == '_') {
            return false
        }
    }
    true
}

/// Emits a basic quoted TOML string, escaping `"`, `\`, and control characters.
func emitTomlString(s: String, mutating buf: String) {
    buf.append("\"");
    for c in s {
        match c {
            '"' => buf.append("\\\""),
            '\\' => buf.append("\\\\"),
            '\n' => buf.append("\\n"),
            '\r' => buf.append("\\r"),
            '\t' => buf.append("\\t"),
            '\u{08}' => buf.append("\\b"),
            _ => buf.append(char: c)
        }
    }
    buf.append("\"")
}
