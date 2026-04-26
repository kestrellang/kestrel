// TOML emitter - converts Value.Object to TOML format

module quill.toml.emitter

import quill.value.(Value)
import quill.error.(SerializeError, SerializeErrorKind)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Emits a Value as a TOML string.
/// The value must be an Object at the top level.
public func emitToml(value: Value) -> Result[String, SerializeError] {
    match value {
        .Obj(obj) => {
            var buf = String();
            emitTable(obj, buf, "");
            .Ok(buf)
        },
        _ => .Err(SerializeError.custom("TOML top-level value must be an object"))
    }
}

// ============================================================================
// TABLE EMITTER
// ============================================================================

/// Emits a table's contents. Non-table values first, then sub-tables with headers.
func emitTable(obj: Dictionary[String, Value], mutating buf: String, prefix: String) {
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
                let fullKey = if prefix.byteCount > 0 {
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
        .Int(n) => buf.append(n.format()),
        .Float(f) => {
            let s = f.format();
            buf.append(s);
            // Ensure float has decimal point
            var hasDot = false;
            var i: Int64 = 0;
            let len = s.byteCount;
            while i < len {
                let b = s.bytes(unchecked: i);
                if b == 46 or b == 101 or b == 69 {
                    hasDot = true
                }
                i = i + 1
            }
            if hasDot == false {
                buf.append(".0")
            }
        },
        .Str(s) => emitTomlString(s, buf),
        .Arr(arr) => {
            buf.append("[");
            var i: Int64 = 0;
            while i < arr.count {
                if i > 0 {
                    buf.append(", ")
                }
                emitTomlValue(arr(unchecked: i), buf);
                i = i + 1
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

/// Emits a TOML key. Uses bare key if possible, quoted otherwise.
func emitKey(key: String, mutating buf: String) {
    if isBareKey(key) {
        buf.append(key)
    } else {
        emitTomlString(key, buf)
    }
}

/// Returns true if the string can be a bare TOML key.
/// Bare keys may only contain A-Za-z0-9, '-', '_'.
func isBareKey(s: String) -> Bool {
    let len = s.byteCount;
    if len == 0 {
        return false
    }
    var i: Int64 = 0;
    while i < len {
        let b = s.bytes(unchecked: i);
        let isAlpha = (b >= 65 and b <= 90) or (b >= 97 and b <= 122);
        let isDigit = b >= 48 and b <= 57;
        let isDash = b == 45;
        let isUnderscore = b == 95;
        if isAlpha or isDigit or isDash or isUnderscore {
            i = i + 1
        } else {
            return false
        }
    }
    true
}

/// Emits a quoted TOML string with escape sequences.
func emitTomlString(s: String, mutating buf: String) {
    buf.append("\"");
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 34 {
            buf.append("\\\"")
        } else if b == 92 {
            buf.append("\\\\")
        } else if b == 10 {
            buf.append("\\n")
        } else if b == 13 {
            buf.append("\\r")
        } else if b == 9 {
            buf.append("\\t")
        } else if b == 8 {
            buf.append("\\b")
        } else {
            buf.appendByte(b)
        }
        i = i + 1
    }
    buf.append("\"")
}
