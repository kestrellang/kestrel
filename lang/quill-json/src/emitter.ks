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
    buf.appendByte(10); // trailing newline
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
            buf.appendByte(91); // '['
            var i: Int64 = 0;
            while i < arr.count {
                if i > 0 {
                    buf.appendByte(44) // ','
                }
                emitValue(arr(unchecked: i), buf);
                i = i + 1
            }
            buf.appendByte(93) // ']'
        },
        .Obj(obj) => {
            buf.appendByte(123); // '{'
            var first = true;
            for (key, val) in obj.iter() {
                if first {
                    first = false
                } else {
                    buf.appendByte(44) // ','
                }
                emitString(key, buf);
                buf.appendByte(58); // ':'
                emitValue(val, buf)
            }
            buf.appendByte(125) // '}'
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
                buf.appendByte(91); // '['
                buf.appendByte(10); // '\n'
                let childIndent = indent + 2;
                var i: Int64 = 0;
                while i < arr.count {
                    if i > 0 {
                        buf.appendByte(44); // ','
                        buf.appendByte(10)  // '\n'
                    }
                    writeIndent(buf, childIndent);
                    emitPretty(arr(unchecked: i), buf, childIndent);
                    i = i + 1
                }
                buf.appendByte(10); // '\n'
                writeIndent(buf, indent);
                buf.appendByte(93) // ']'
            }
        },
        .Obj(obj) => {
            if obj.isEmpty {
                buf.append("{}")
            } else {
                buf.appendByte(123); // '{'
                buf.appendByte(10);  // '\n'
                let childIndent = indent + 2;
                var first = true;
                for (key, val) in obj.iter() {
                    if first {
                        first = false
                    } else {
                        buf.appendByte(44); // ','
                        buf.appendByte(10)  // '\n'
                    }
                    writeIndent(buf, childIndent);
                    emitString(key, buf);
                    buf.append(": ");
                    emitPretty(val, buf, childIndent)
                }
                buf.appendByte(10); // '\n'
                writeIndent(buf, indent);
                buf.appendByte(125) // '}'
            }
        }
    }
}

func writeIndent(mutating buf: String, count: Int64) {
    var i: Int64 = 0;
    while i < count {
        buf.appendByte(32); // space
        i = i + 1
    }
}

// ============================================================================
// STRING ESCAPING
// ============================================================================

/// Emits a properly escaped JSON string including surrounding quotes.
func emitString(s: String, mutating buf: String) {
    buf.appendByte(34); // '"'
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.byteAtUnchecked(i);
        let n = Int64(from: b);
        if n == 34 { // '"'
            buf.appendByte(92); // '\'
            buf.appendByte(34)
        } else if n == 92 { // '\'
            buf.appendByte(92);
            buf.appendByte(92)
        } else if n == 8 { // backspace
            buf.appendByte(92);
            buf.appendByte(98)
        } else if n == 12 { // form feed
            buf.appendByte(92);
            buf.appendByte(102)
        } else if n == 10 { // newline
            buf.appendByte(92);
            buf.appendByte(110)
        } else if n == 13 { // carriage return
            buf.appendByte(92);
            buf.appendByte(114)
        } else if n == 9 { // tab
            buf.appendByte(92);
            buf.appendByte(116)
        } else if n < 32 { // other control characters
            buf.append("\\u00");
            buf.appendByte(hexChar(n / 16));
            buf.appendByte(hexChar(n % 16))
        } else {
            buf.appendByte(b)
        }
        i = i + 1
    }
    buf.appendByte(34) // '"'
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
        let b = Int64(from: s.byteAtUnchecked(i));
        if b == 46 or b == 101 or b == 69 { // '.' or 'e' or 'E'
            hasDot = true
        }
        i = i + 1
    }
    if hasDot == false {
        buf.append(".0")
    }
}
