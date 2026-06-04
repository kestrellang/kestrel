/// Converts a quill `Value` tree into a JSON string.
///
/// Provides two public entry points: `emitJson` for compact single-line
/// output (no whitespace between tokens) and `emitJsonPretty` for
/// human-readable 2-space-indented output with a trailing newline.

module quill.json.emitter

import quill.value.(Value)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Converts a `Value` to a compact JSON string with no extra whitespace.
///
/// Every `Value` variant maps directly to a JSON production — there is no
/// lossy conversion. The output contains no indentation, no trailing
/// newline, and no spaces around `:` or `,`.
///
/// # Examples
///
/// ```
/// let v = Value.Obj([("x", Value.Int(1)), ("y", Value.Arr([Value.Null]))]);
/// emitJson(v);  // "{\"x\":1,\"y\":[null]}"
/// ```
public func emitJson(value: Value) -> String {
    var buf = String();
    emitValue(value, buf);
    buf
}

/// Converts a `Value` to a pretty-printed JSON string with 2-space indentation.
///
/// Identical semantics to `emitJson` but inserts newlines between array
/// elements and object entries, indents nested structures by 2 spaces per
/// level, and places a space after `:` in object entries. The output ends
/// with a single trailing newline.
///
/// # Examples
///
/// ```
/// let v = Value.Obj([("name", Value.Str("Alice"))]);
/// emitJsonPretty(v);  // "{\n  \"name\": \"Alice\"\n}\n"
/// ```
public func emitJsonPretty(value: Value) -> String {
    var buf = String();
    emitPretty(value, buf, 0);
    buf.append("\n");
    buf
}

// ============================================================================
// COMPACT EMITTER
// ============================================================================

/// Recursively emits a single value in compact form (no whitespace).
func emitValue(value: Value, mutating buf: String) {
    match value {
        .Null => buf.append("null"),
        .Boolean(b) => {
            if b {
                buf.append("true")
            } else {
                buf.append("false")
            }
        },
        .Int(n) => buf.append("\(n)"),
        .Float(f) => emitFloat(f, buf),
        .Str(s) => emitString(s, buf),
        .Arr(arr) => {
            buf.append("[");
            var i: Int64 = 0;
            while i < arr.count {
                if i > 0 {
                    buf.append(",")
                }
                emitValue(arr(unchecked: i), buf);
                i = i + 1
            }
            buf.append("]")
        },
        .Obj(obj) => {
            buf.append("{");
            var first = true;
            for (key, val) in obj.iter() {
                if first {
                    first = false
                } else {
                    buf.append(",")
                }
                emitString(key, buf);
                buf.append(":");
                emitValue(val, buf)
            }
            buf.append("}")
        }
    }
}

// ============================================================================
// PRETTY EMITTER
// ============================================================================

/// Recursively emits a single value with 2-space indentation per level.
func emitPretty(value: Value, mutating buf: String, indent: Int64) {
    match value {
        .Null => buf.append("null"),
        .Boolean(b) => {
            if b {
                buf.append("true")
            } else {
                buf.append("false")
            }
        },
        .Int(n) => buf.append("\(n)"),
        .Float(f) => emitFloat(f, buf),
        .Str(s) => emitString(s, buf),
        .Arr(arr) => {
            if arr.isEmpty {
                buf.append("[]")
            } else {
                buf.append("[");
                buf.append("\n");
                let childIndent = indent + 2;
                var i: Int64 = 0;
                while i < arr.count {
                    if i > 0 {
                        buf.append(",");
                        buf.append("\n")
                    }
                    writeIndent(buf, childIndent);
                    emitPretty(arr(unchecked: i), buf, childIndent);
                    i = i + 1
                }
                buf.append("\n");
                writeIndent(buf, indent);
                buf.append("]")
            }
        },
        .Obj(obj) => {
            if obj.isEmpty {
                buf.append("{}")
            } else {
                buf.append("{");
                buf.append("\n");
                let childIndent = indent + 2;
                var first = true;
                for (key, val) in obj.iter() {
                    if first {
                        first = false
                    } else {
                        buf.append(",");
                        buf.append("\n")
                    }
                    writeIndent(buf, childIndent);
                    emitString(key, buf);
                    buf.append(": ");
                    emitPretty(val, buf, childIndent)
                }
                buf.append("\n");
                writeIndent(buf, indent);
                buf.append("}")
            }
        }
    }
}

/// Appends `count` space characters to the buffer.
func writeIndent(mutating buf: String, count: Int64) {
    var i: Int64 = 0;
    while i < count {
        buf.append(" ");
        i = i + 1
    }
}

// ============================================================================
// STRING ESCAPING
// ============================================================================

/// Emits a JSON string with surrounding quotes and all required escapes.
///
/// Escapes `"`, `\`, and control characters (U+0000–U+001F). Control
/// characters without a dedicated escape sequence use `\u00XX` form.
/// Non-ASCII bytes pass through unchanged (valid UTF-8 is already legal
/// in JSON strings).
func emitString(s: String, mutating buf: String) {
    buf.append("\"");
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 34 {
            buf.append("\\\"")
        } else if b == 92 {
            buf.append("\\\\")
        } else if b == 8 {
            buf.append("\\b")
        } else if b == 12 {
            buf.append("\\f")
        } else if b == 10 {
            buf.append("\\n")
        } else if b == 13 {
            buf.append("\\r")
        } else if b == 9 {
            buf.append("\\t")
        } else if b < 32 {
            buf.append("\\u00\(b:02x)")
        } else {
            buf.append(char: Char(UInt32(from: b)).unwrap())
        }
        i = i + 1
    }
    buf.append("\"")
}

/// Returns the lowercase hex digit (`0`–`f`) for a value 0–15.
func hexChar(n: Int64) -> UInt8 {
    if n < 10 {
        UInt8(from: n + 48) // '0' + n
    } else {
        UInt8(from: n + 87) // 'a' + (n - 10)
    }
}

// ============================================================================
// FLOAT FORMATTING
// ============================================================================

/// Emits a float, appending `.0` when the formatted representation lacks
/// a decimal point or exponent (so `3.0` is never confused with integer `3`).
func emitFloat(f: Float64, mutating buf: String) {
    let s = "\(f)";
    buf.append(s);
    // Check if the formatted string contains a '.' or 'e'
    // If not, append ".0" to distinguish from integers
    var hasDot = false;
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        let dot: UInt8 = 46;
        let eL: UInt8 = 101;
        let eU: UInt8 = 69;
        if b == dot or b == eL or b == eU {
            hasDot = true
        }
        i = i + 1
    }
    if not hasDot {
        buf.append(".0")
    }
}
