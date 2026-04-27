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

    var lines = splitLines(source);
    var lineIdx: Int64 = 0;

    while lineIdx < lines.count {
        let rawLine = lines(unchecked: lineIdx);
        let line = rawLine.trimmed();
        lineIdx = lineIdx + 1;

        // Skip empty lines and comments
        if line.isEmpty or line.starts(with: "#") {
            lineNum = lineNum + 1;
            continue
        }

        // Table header [section]
        if line.starts(with: "[") {
            // Array of tables [[ ... ]] not supported
            if line.starts(with: "[[") {
                return .Err(TomlParseError("array of tables [[...]] not supported", lineNum))
            }

            // Find closing bracket
            match line.find(matching: { (c) in c == ']' }) {
                .Some(endPos) => {
                    currentTable = line.substringBytes(from: 1, to: endPos).trimmed();
                    ensureTable(root, currentTable);
                    lineNum = lineNum + 1;
                    continue
                },
                .None => return .Err(TomlParseError("unterminated table header", lineNum))
            }
        }

        // Key = value pair
        let parsed = try parseKeyValue(line, lineNum);
        let key = parsed.0;
        let val = parsed.1;

        if currentTable.isEmpty {
            let _ = root.insert(key, val);
        } else {
            insertIntoTable(root, currentTable, key, val);
        }

        lineNum = lineNum + 1
    }

    .Ok(Value.Obj(root))
}

// ============================================================================
// LINE SPLITTING
// ============================================================================

/// Splits source into logical lines, recognising `\n`, `\r\n`, and `\r`.
/// Materializes the `s.lines` view so the caller can index into the result.
func splitLines(s: String) -> Array[String] {
    var lines = Array[String]();
    var iter = s.lines.iter();
    while let .Some(line) = iter.next() {
        lines.append(line)
    }
    lines
}

// ============================================================================
// KEY-VALUE PARSING
// ============================================================================

func parseKeyValue(line: String, lineNum: Int64) -> Result[(String, Value), TomlParseError] {
    // Find the '=' sign, respecting double-quoted strings.
    var inQuote = false;
    var found: Optional[Int64] = .None;
    var iter = line.chars.iter();
    var pos: Int64 = 0;
    while let .Some(c) = iter.next() {
        if c == '"' {
            inQuote = inQuote == false
        } else if inQuote == false and c == '=' {
            found = .Some(pos);
            break
        }
        pos = pos + c.utf8Length()
    }

    let eqPos = match found {
        .Some(p) => p,
        .None => return .Err(TomlParseError("expected '=' in key-value pair", lineNum))
    };

    let len = line.byteCount;
    let rawKey = line.substringBytes(from: 0, to: eqPos).trimmed();
    let rawVal = line.substringBytes(from: eqPos + 1, to: len).trimmed();

    // Strip comment from value (not inside strings)
    let valStr = stripInlineComment(rawVal);

    let key = parseKey(rawKey);
    let value = try parseTomlValue(valStr, lineNum);

    .Ok((key, value))
}

/// Parses a TOML key — bare keys or basic quoted strings.
func parseKey(s: String) -> String {
    if s.byteCount >= 2 and s.starts(with: "\"") and s.ends(with: "\"") {
        return s.substringBytes(from: 1, to: s.byteCount - 1)
    }
    s
}

/// Strips an inline comment (`# ...`) from a value string, respecting quotes.
func stripInlineComment(s: String) -> String {
    var inQuote = false;
    var iter = s.chars.iter();
    var pos: Int64 = 0;
    while let .Some(c) = iter.next() {
        if c == '"' {
            inQuote = inQuote == false
        } else if inQuote == false and c == '#' {
            return s.substringBytes(from: 0, to: pos).trimmed()
        }
        pos = pos + c.utf8Length()
    }
    s
}

// ============================================================================
// VALUE PARSING
// ============================================================================

func parseTomlValue(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    if s.isEmpty {
        return .Err(TomlParseError("empty value", lineNum))
    }

    // String: "..."
    if s.starts(with: "\"") {
        let str = try parseTomlString(s, lineNum);
        return .Ok(Value.Str(str))
    }

    // Array: [...]
    if s.starts(with: "[") {
        return parseTomlArray(s, lineNum)
    }

    // Inline table: {...}
    if s.starts(with: "{") {
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
    if s.byteCount < 2 or not s.ends(with: "\"") {
        return .Err(TomlParseError("unterminated string", lineNum))
    }

    var result = String();
    var iter = s.substringBytes(from: 1, to: s.byteCount - 1).chars.iter();
    while let .Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                .Some(esc) => {
                    if esc == '"' {
                        result.appendChar('"')
                    } else if esc == '\\' {
                        result.appendChar('\\')
                    } else if esc == 'n' {
                        result.appendChar('\n')
                    } else if esc == 't' {
                        result.appendChar('\t')
                    } else if esc == 'r' {
                        result.appendChar('\r')
                    } else {
                        return .Err(TomlParseError("invalid escape sequence", lineNum))
                    }
                },
                .None => return .Err(TomlParseError("unterminated escape in string", lineNum))
            }
        } else {
            result.appendChar(c)
        }
    }

    .Ok(result)
}

func parseTomlNumber(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    // Float if it contains a decimal point or exponent
    let isFloat = s.contains(matching: { (c) in c == '.' or c == 'e' or c == 'E' });

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
    if not s.ends(with: "]") {
        return .Err(TomlParseError("unterminated array", lineNum))
    }

    let inner = s.substringBytes(from: 1, to: s.byteCount - 1).trimmed();
    if inner.isEmpty {
        return .Ok(Value.Arr(Array[Value]()))
    }

    var items = Array[Value]();
    var parts = splitTomlArray(inner);
    var pi: Int64 = 0;
    while pi < parts.count {
        let part = parts(unchecked: pi).trimmed();
        if not part.isEmpty {
            let val = try parseTomlValue(part, lineNum);
            items.append(val)
        }
        pi = pi + 1
    }

    .Ok(Value.Arr(items))
}

/// Parses an inline table: `{ key = value, key2 = value2 }`
func parseInlineTable(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    if not s.ends(with: "}") {
        return .Err(TomlParseError("unterminated inline table", lineNum))
    }

    let inner = s.substringBytes(from: 1, to: s.byteCount - 1).trimmed();
    if inner.isEmpty {
        return .Ok(Value.Obj(Dictionary[String, Value]()))
    }

    var obj = Dictionary[String, Value]();
    var parts = splitTomlArray(inner);
    var pi: Int64 = 0;
    while pi < parts.count {
        let part = parts(unchecked: pi).trimmed();
        if not part.isEmpty {
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
    var pos: Int64 = 0;
    var iter = s.chars.iter();
    while let .Some(c) = iter.next() {
        if c == '"' {
            inQuote = inQuote == false
        } else if inQuote == false {
            if c == '[' or c == '{' {
                depth = depth + 1
            } else if c == ']' or c == '}' {
                depth = depth - 1
            } else if c == ',' and depth == 0 {
                parts.append(s.substringBytes(from: start, to: pos));
                start = pos + c.utf8Length()
            }
        }
        pos = pos + c.utf8Length()
    }

    if start < s.byteCount {
        parts.append(s.substringBytes(from: start, to: s.byteCount))
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
// NUMBER PARSING
// ============================================================================

/// Parses a TOML integer (supports optional leading sign and underscores).
func tomlParseInt(s: String) -> Optional[Int64] {
    if s.isEmpty {
        return .None
    }

    var iter = s.chars.iter();
    var negative = false;
    var first = match iter.next() {
        .Some(c) => c,
        .None => return .None
    };

    if first == '-' {
        negative = true;
        match iter.next() {
            .Some(c) => first = c,
            .None => return .None
        }
    } else if first == '+' {
        match iter.next() {
            .Some(c) => first = c,
            .None => return .None
        }
    }

    var result: Int64 = 0;
    var current: Optional[Char] = .Some(first);
    while let .Some(c) = current {
        if c == '_' {
            // TOML allows underscore separators
        } else if let .Some(d) = c.digitValue() {
            result = result * 10 + Int64(from: d)
        } else {
            return .None
        }
        current = iter.next()
    }

    if negative { .Some(0 - result) } else { .Some(result) }
}

/// Parses a TOML float (supports underscores, inf, nan).
func tomlParseFloat(s: String) -> Optional[Float64] {
    if s == "inf" or s == "+inf" {
        return .Some(Float64.infinity)
    }
    if s == "-inf" {
        return .Some(0.0 - Float64.infinity)
    }
    if s == "nan" or s == "+nan" or s == "-nan" {
        return .Some(Float64.nan)
    }

    // Strip underscores, then parse via shared float scanner.
    var cleaned = String();
    var iter = s.chars.iter();
    while let .Some(c) = iter.next() {
        if c != '_' {
            cleaned.appendChar(c)
        }
    }

    parseFloat(cleaned)
}

/// Shared float scanner — integer part, fractional part, exponent.
func parseFloat(s: String) -> Optional[Float64] {
    if s.isEmpty {
        return .None
    }

    var iter = s.chars.iter();
    var negative = false;
    var pending = match iter.next() {
        .Some(c) => c,
        .None => return .None
    };

    if pending == '-' {
        negative = true;
        match iter.next() {
            .Some(c) => pending = c,
            .None => return .None
        }
    } else if pending == '+' {
        match iter.next() {
            .Some(c) => pending = c,
            .None => return .None
        }
    }

    var current: Optional[Char] = .Some(pending);

    // Integer part
    var intPart: Float64 = 0.0;
    while let .Some(c) = current {
        match c.digitValue() {
            .Some(d) => {
                intPart = intPart * 10.0 + Float64(from: Int64(from: d));
                current = iter.next()
            },
            .None => break
        }
    }

    // Fractional part
    var fracPart: Float64 = 0.0;
    var fracDiv: Float64 = 1.0;
    if let .Some(c) = current {
        if c == '.' {
            current = iter.next();
            while let .Some(d) = current {
                match d.digitValue() {
                    .Some(v) => {
                        fracPart = fracPart * 10.0 + Float64(from: Int64(from: v));
                        fracDiv = fracDiv * 10.0;
                        current = iter.next()
                    },
                    .None => break
                }
            }
        }
    }

    var result = intPart + fracPart / fracDiv;

    // Exponent
    if let .Some(c) = current {
        if c == 'e' or c == 'E' {
            current = iter.next();
            var expNeg = false;
            if let .Some(s) = current {
                if s == '+' {
                    current = iter.next()
                } else if s == '-' {
                    expNeg = true;
                    current = iter.next()
                }
            }
            var exp: Float64 = 0.0;
            while let .Some(d) = current {
                match d.digitValue() {
                    .Some(v) => {
                        exp = exp * 10.0 + Float64(from: Int64(from: v));
                        current = iter.next()
                    },
                    .None => break
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
