// JSON emitter - converts Value to JSON strings

module quill.json.emitter

import quill.value.(Value)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Emits a Value as a compact JSON string with no extra whitespace.
public func emitJson(value: Value) -> String {
    var buf = String();
    emitValue(value, buf);
    buf
}

/// Emits a Value as a pretty-printed JSON string with 2-space indentation.
public func emitJsonPretty(value: Value) -> String {
    var buf = String();
    emitPretty(value, buf, 0);
    buf.append("\n");
    buf
}

// ============================================================================
// COMPACT EMITTER
// ============================================================================

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
        .Int(n) => buf.append(n.format()),
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
        .Int(n) => buf.append(n.format()),
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

/// Emits a properly escaped JSON string including surrounding quotes.
func emitString(s: String, mutating buf: String) {
    buf.append("\"");
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.byteAtUnchecked(i);
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
            buf.append("\\u00");
            let n = Int64(from: b);
            buf.appendByte(hexChar(n / 16));
            buf.appendByte(hexChar(n % 16))
        } else {
            buf.appendByte(b)
        }
        i = i + 1
    }
    buf.append("\"")
}

/// Returns the hex character for a value 0-15.
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

/// Emits a float value. Ensures integer-valued floats still have ".0".
func emitFloat(f: Float64, mutating buf: String) {
    let s = f.format();
    buf.append(s);
    // Check if the formatted string contains a '.' or 'e'
    // If not, append ".0" to distinguish from integers
    var hasDot = false;
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.byteAtUnchecked(i);
        if b == 46 or b == 101 or b == 69 {
            hasDot = true
        }
        i = i + 1
    }
    if hasDot == false {
        buf.append(".0")
    }
}
