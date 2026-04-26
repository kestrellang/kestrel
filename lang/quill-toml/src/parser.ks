// TOML parser - line-oriented, minimal subset for flock.toml
//
// Supported: bare keys, basic quoted strings, integers, floats, bools,
//            arrays, inline tables, standard tables [section]
// Not supported: datetime, array of tables [[section]],
//                multiline strings, dotted keys, literal strings

module quill.toml.parser

import quill.value.(Value)
import quill.toml.error.(TomlParseError)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Parses a TOML string into a Value.Object.
public func parseToml(source: String) -> Result[Value, TomlParseError] {
    var root = Dictionary[String, Value]();
    var currentTable = "";
    var lineNum: Int64 = 1;

    // Split into lines manually
    var lines = splitLines(source);
    var lineIdx: Int64 = 0;

    while lineIdx < lines.count {
        let rawLine = lines(unchecked: lineIdx);
        let line = trimWhitespace(rawLine);
        lineIdx = lineIdx + 1;

        // Skip empty lines and comments
        if line.byteCount == 0 {
            lineNum = lineNum + 1;
            continue
        }
        let firstByte = line.bytes(unchecked: 0);
        if firstByte == 35 {
            lineNum = lineNum + 1;
            continue
        }

        // Table header [section]
        if firstByte == 91 {
            // Check for array of tables [[
            if line.byteCount > 1 {
                let secondByte = line.bytes(unchecked: 1);
                if secondByte == 91 {
                    return .Err(TomlParseError("array of tables [[...]] not supported", lineNum))
                }
            }
            // Find closing bracket
            var endPos: Int64 = 1;
            let lineLen = line.byteCount;
            while endPos < lineLen {
                let b = line.bytes(unchecked: endPos);
                if b == 93 {
                    break
                }
                endPos = endPos + 1
            }
            if endPos >= lineLen {
                return .Err(TomlParseError("unterminated table header", lineNum))
            }
            currentTable = trimWhitespace(line.substringBytes(from: 1, to: endPos));

            // Ensure the table exists in root
            ensureTable(root, currentTable);
            lineNum = lineNum + 1;
            continue
        }

        // Key = value pair
        let parsed = try parseKeyValue(line, lineNum);
        let key = parsed.0;
        let val = parsed.1;

        if currentTable.byteCount == 0 {
            // Top-level
            let _ = root.insert(key, val);
        } else {
            // Insert into the appropriate table
            insertIntoTable(root, currentTable, key, val);
        }

        lineNum = lineNum + 1
    }

    .Ok(Value.Obj(root))
}

// ============================================================================
// LINE SPLITTING
// ============================================================================

func splitLines(s: String) -> Array[String] {
    var lines = Array[String]();
    var start: Int64 = 0;
    var i: Int64 = 0;
    let len = s.byteCount;

    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 10 {
            lines.append(s.substringBytes(from: start, to: i));
            start = i + 1
        } else if b == 13 {
            lines.append(s.substringBytes(from: start, to: i));
            // Handle \r\n
            if i + 1 < len {
                let next = s.bytes(unchecked: i + 1);
                if next == 10 {
                    i = i + 1
                }
            }
            start = i + 1
        }
        i = i + 1
    }

    // Last line (if no trailing newline)
    if start <= len {
        lines.append(s.substringBytes(from: start, to: len))
    }

    lines
}

// ============================================================================
// KEY-VALUE PARSING
// ============================================================================

func parseKeyValue(line: String, lineNum: Int64) -> Result[(String, Value), TomlParseError] {
    // Find the '=' sign
    var eqPos: Int64 = 0;
    let len = line.byteCount;
    var foundEq = false;
    var inQuote = false;

    while eqPos < len {
        let b = line.bytes(unchecked: eqPos);
        if b == 34 {
            inQuote = inQuote == false
        }
        if inQuote == false and b == 61 {
            foundEq = true;
            break
        }
        eqPos = eqPos + 1
    }

    if foundEq == false {
        return .Err(TomlParseError("expected '=' in key-value pair", lineNum))
    }

    let rawKey = trimWhitespace(line.substringBytes(from: 0, to: eqPos));
    let rawVal = trimWhitespace(line.substringBytes(from: eqPos + 1, to: len));

    // Strip comment from value (not inside strings)
    let valStr = stripInlineComment(rawVal);

    // Parse the key (bare or quoted)
    let key = parseKey(rawKey);

    // Parse the value
    let value = try parseTomlValue(valStr, lineNum);

    .Ok((key, value))
}

/// Parses a TOML key - bare keys or basic quoted strings.
func parseKey(s: String) -> String {
    let len = s.byteCount;
    if len >= 2 {
        let first = s.bytes(unchecked: 0);
        let last = s.bytes(unchecked: len - 1);
        if first == 34 and last == 34 { // quoted key
            return s.substringBytes(from: 1, to: len - 1)
        }
    }
    s
}

/// Strips an inline comment (# ...) from a value string, respecting quotes.
func stripInlineComment(s: String) -> String {
    var inQuote = false;
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 34 {
            inQuote = inQuote == false;
        }
        if inQuote == false and b == 35 {
            return trimWhitespace(s.substringBytes(from: 0, to: i))
        }
        i = i + 1
    }
    s
}

// ============================================================================
// VALUE PARSING
// ============================================================================

func parseTomlValue(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    let len = s.byteCount;
    if len == 0 {
        return .Err(TomlParseError("empty value", lineNum))
    }

    let firstByte = s.bytes(unchecked: 0);

    // String: "..."
    if firstByte == 34 {
        let str = try parseTomlString(s, lineNum);
        return .Ok(Value.Str(str))
    }

    // Array: [...]
    if firstByte == 91 {
        return parseTomlArray(s, lineNum)
    }

    // Inline table: {...}
    if firstByte == 123 {
        return parseInlineTable(s, lineNum)
    }

    // Boolean
    if s == "true" {
        return .Ok(Value.Boolean(true))
    }
    if s == "false" {
        return .Ok(Value.Boolean(false))
    }

    // Number (integer or float)
    parseTomlNumber(s, lineNum)
}

func parseTomlString(s: String, lineNum: Int64) -> Result[String, TomlParseError] {
    let len = s.byteCount;
    if len < 2 {
        return .Err(TomlParseError("unterminated string", lineNum))
    }
    let last = s.bytes(unchecked: len - 1);
    if last != 34 {
        return .Err(TomlParseError("unterminated string", lineNum))
    }

    var result = String();
    var i: Int64 = 1;
    let end = len - 1;

    while i < end {
        let b = s.bytes(unchecked: i);
        if b == 92 {
            i = i + 1;
            if i >= end {
                return .Err(TomlParseError("unterminated escape in string", lineNum))
            }
            let esc = s.bytes(unchecked: i);
            if esc == 34 { // \"
                result.appendByte(34)
            } else if esc == 92 { // \\
                result.appendByte(92)
            } else if esc == 110 { // \n
                result.appendByte(10)
            } else if esc == 116 { // \t
                result.appendByte(9)
            } else if esc == 114 { // \r
                result.appendByte(13)
            } else {
                return .Err(TomlParseError("invalid escape sequence", lineNum))
            }
        } else {
            result.appendByte(b)
        }
        i = i + 1
    }

    .Ok(result)
}

func parseTomlNumber(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    // Check if it's a float (contains '.' or 'e'/'E')
    var isFloat = false;
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 46 or b == 101 or b == 69 {
            isFloat = true
        }
        i = i + 1
    }

    if isFloat {
        match tomlParseFloat(s) {
            .Some(f) => .Ok(Value.Float(f)),
            .None => .Err(TomlParseError("invalid float: " + s, lineNum))
        }
    } else {
        match tomlParseInt(s) {
            .Some(n) => .Ok(Value.Int(n)),
            .None => .Err(TomlParseError("invalid integer: " + s, lineNum))
        }
    }
}

func parseTomlArray(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    let len = s.byteCount;
    // Verify brackets
    let lastByte = s.bytes(unchecked: len - 1);
    if lastByte != 93 {
        return .Err(TomlParseError("unterminated array", lineNum))
    }

    let inner = trimWhitespace(s.substringBytes(from: 1, to: len - 1));
    if inner.byteCount == 0 {
        return .Ok(Value.Arr(Array[Value]()))
    }

    // Split by commas (respecting quotes and nested brackets)
    var items = Array[Value]();
    var parts = splitTomlArray(inner);
    var pi: Int64 = 0;
    while pi < parts.count {
        let part = trimWhitespace(parts(unchecked: pi));
        if part.byteCount > 0 {
            let val = try parseTomlValue(part, lineNum);
            items.append(val)
        }
        pi = pi + 1
    }

    .Ok(Value.Arr(items))
}

/// Parses an inline table: { key = value, key2 = value2 }
func parseInlineTable(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    let len = s.byteCount;
    let lastByte = s.bytes(unchecked: len - 1);
    if lastByte != 125 {
        return .Err(TomlParseError("unterminated inline table", lineNum))
    }

    let inner = trimWhitespace(s.substringBytes(from: 1, to: len - 1));
    if inner.byteCount == 0 {
        return .Ok(Value.Obj(Dictionary[String, Value]()))
    }

    // Split by commas (respecting quotes and nesting)
    var obj = Dictionary[String, Value]();
    var parts = splitTomlArray(inner);
    var pi: Int64 = 0;
    while pi < parts.count {
        let part = trimWhitespace(parts(unchecked: pi));
        if part.byteCount > 0 {
            let kv = try parseKeyValue(part, lineNum);
            let _ = obj.insert(kv.0, kv.1);
        }
        pi = pi + 1
    }

    .Ok(Value.Obj(obj))
}

/// Splits array contents by commas, respecting quotes and nesting.
func splitTomlArray(s: String) -> Array[String] {
    var parts = Array[String]();
    var depth: Int64 = 0;
    var inQuote = false;
    var start: Int64 = 0;
    var i: Int64 = 0;
    let len = s.byteCount;

    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 34 {
            inQuote = inQuote == false
        } else if inQuote == false {
            if b == 91 or b == 123 {
                depth = depth + 1
            } else if b == 93 or b == 125 {
                depth = depth - 1
            } else if b == 44 and depth == 0 {
                parts.append(s.substringBytes(from: start, to: i));
                start = i + 1
            }
        }
        i = i + 1
    }

    // Last element
    if start < len {
        parts.append(s.substringBytes(from: start, to: len))
    }

    parts
}

// ============================================================================
// TABLE MANAGEMENT
// ============================================================================

/// Ensures a table exists in the root dictionary.
func ensureTable(mutating root: Dictionary[String, Value], name: String) {
    match root(name) {
        .Some(_) => {},
        .None => { let _ = root.insert(name, Value.Obj(Dictionary[String, Value]())); }
    }
}

/// Inserts a key-value pair into a named table.
func insertIntoTable(mutating root: Dictionary[String, Value], table: String, key: String, value: Value) {
    match root(table) {
        .Some(existing) => {
            match existing {
                .Obj(obj) => {
                    var mutObj = obj;
                    let _ = mutObj.insert(key, value);
                    let _ = root.insert(table, Value.Obj(mutObj));
                },
                _ => {
                    // Table exists but is not an object - overwrite
                    var newObj = Dictionary[String, Value]();
                    let _ = newObj.insert(key, value);
                    let _ = root.insert(table, Value.Obj(newObj));
                }
            }
        },
        .None => {
            var newObj = Dictionary[String, Value]();
            let _ = newObj.insert(key, value);
            let _ = root.insert(table, Value.Obj(newObj));
        }
    }
}

// ============================================================================
// STRING UTILITIES
// ============================================================================

/// Trims leading and trailing whitespace from a string.
func trimWhitespace(s: String) -> String {
    let len = s.byteCount;
    if len == 0 {
        return s
    }

    var start: Int64 = 0;
    while start < len {
        let b = s.bytes(unchecked: start);
        if b == 32 or b == 9 or b == 10 or b == 13 {
            start = start + 1
        } else {
            break
        }
    }

    var end = len;
    while end > start {
        let b = s.bytes(unchecked: end - 1);
        if b == 32 or b == 9 or b == 10 or b == 13 {
            end = end - 1
        } else {
            break
        }
    }

    if start == 0 and end == len {
        s
    } else {
        s.substringBytes(from: start, to: end)
    }
}

// ============================================================================
// NUMBER PARSING
// ============================================================================

/// Parses a TOML integer (supports optional leading sign and underscores).
func tomlParseInt(s: String) -> Optional[Int64] {
    let len = s.byteCount;
    if len == 0 {
        return .None
    }

    var i: Int64 = 0;
    var negative = false;
    let first = Int64(from: s.bytes(unchecked: 0));
    if first == 45 { // '-'
        negative = true;
        i = 1
    } else if first == 43 { // '+'
        i = 1
    }

    if i >= len {
        return .None
    }

    var result: Int64 = 0;
    while i < len {
        let b = Int64(from: s.bytes(unchecked: i));
        if b == 95 { // '_' - TOML allows underscores in numbers
            i = i + 1;
            continue
        }
        if b < 48 or b > 57 {
            return .None
        }
        result = result * 10 + (b - 48);
        i = i + 1
    }

    if negative {
        .Some(0 - result)
    } else {
        .Some(result)
    }
}

/// Parses a TOML float (supports underscores, inf, nan).
func tomlParseFloat(s: String) -> Optional[Float64] {
    // Handle special values
    if s == "inf" or s == "+inf" {
        return .Some(Float64.infinity)
    }
    if s == "-inf" {
        return .Some(0.0 - Float64.infinity)
    }
    if s == "nan" or s == "+nan" or s == "-nan" {
        return .Some(Float64.nan)
    }

    // Strip underscores and parse
    var cleaned = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b != 95 {
            cleaned.appendByte(b)
        }
        i = i + 1
    }

    // Reuse the JSON float parser logic
    let cLen = cleaned.byteCount;
    if cLen == 0 {
        return .None
    }

    var pos: Int64 = 0;
    var negative = false;
    let firstByte = Int64(from: cleaned.bytes(unchecked: 0));
    if firstByte == 45 { // '-'
        negative = true;
        pos = 1
    } else if firstByte == 43 { // '+'
        pos = 1
    }

    // Integer part
    var intPart: Float64 = 0.0;
    while pos < cLen {
        let b = Int64(from: cleaned.bytes(unchecked: pos));
        if b >= 48 and b <= 57 {
            intPart = intPart * 10.0 + Float64(from: b - 48);
            pos = pos + 1
        } else {
            break
        }
    }

    // Fractional part
    var fracPart: Float64 = 0.0;
    var fracDiv: Float64 = 1.0;
    if pos < cLen {
        let dotByte = Int64(from: cleaned.bytes(unchecked: pos));
        if dotByte == 46 {
            pos = pos + 1;
            while pos < cLen {
                let b = Int64(from: cleaned.bytes(unchecked: pos));
                if b >= 48 and b <= 57 {
                    fracPart = fracPart * 10.0 + Float64(from: b - 48);
                    fracDiv = fracDiv * 10.0;
                    pos = pos + 1
                } else {
                    break
                }
            }
        }
    }

    var result = intPart + fracPart / fracDiv;

    // Exponent
    if pos < cLen {
        let eByte = Int64(from: cleaned.bytes(unchecked: pos));
        if eByte == 101 or eByte == 69 {
            pos = pos + 1;
            var expNeg = false;
            if pos < cLen {
                let signByte = Int64(from: cleaned.bytes(unchecked: pos));
                if signByte == 43 {
                    pos = pos + 1
                } else if signByte == 45 {
                    expNeg = true;
                    pos = pos + 1
                }
            }
            var exp: Float64 = 0.0;
            while pos < cLen {
                let b = Int64(from: cleaned.bytes(unchecked: pos));
                if b >= 48 and b <= 57 {
                    exp = exp * 10.0 + Float64(from: b - 48);
                    pos = pos + 1
                } else {
                    break
                }
            }
            var multiplier: Float64 = 1.0;
            var e: Int64 = 0;
            let expInt = match exp.toInt64() {
                .Some(n) => n,
                .None => 0
            };
            while e < expInt {
                multiplier = multiplier * 10.0;
                e = e + 1
            }
            if expNeg {
                result = result / multiplier
            } else {
                result = result * multiplier
            }
        }
    }

    if negative {
        result = 0.0 - result
    }

    .Some(result)
}
