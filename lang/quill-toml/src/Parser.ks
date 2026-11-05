/// Line-oriented TOML parser for the quill framework.
///
/// Parses the subset of TOML used by `flock.toml` configuration files:
/// bare keys, basic quoted strings, integers, floats, booleans, arrays,
/// inline tables, and standard `[section]` tables.
///
/// Not supported: datetime, array of tables `[[...]]`, multiline strings,
/// dotted keys, literal strings (`'...'`).
///
/// # Examples
///
/// ```
/// import quill.toml.parser.(parseToml)
///
/// let v = try parseToml("name = \"hello\"\nversion = \"1.0\"");
/// // v == Value.Obj([("name", Value.Str("hello")), ("version", Value.Str("1.0"))])
/// ```

module quill.toml.parser

import quill.value.(Value)
import quill.toml.error.(TomlParseError)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Parses a TOML document into a `Value.Obj`.
///
/// Processes the source line-by-line. Lines beginning with `[` are table
/// headers; other non-empty, non-comment lines are key-value pairs inserted
/// into the current table.
///
/// # Examples
///
/// ```
/// let v = try parseToml("[package]\nname = \"hello\"");
/// // v == Value.Obj([("package", Value.Obj([("name", Value.Str("hello"))]))])
/// ```
///
/// # Errors
///
/// Throws `TomlParseError` for syntax violations, with a line number
/// pointing at the problem line.
public func parseToml(source: String) -> Value throws TomlParseError {
    var root: [String: Value] = [:];
    var currentTable = "";

    for (index, rawLine) in source.lines.iter().enumerate() {
        let lineNum = index + 1;
        let line = rawLine.trimmed().toOwned();

        // Skip empty lines and comments
        guard let some first = line.chars(checked: 0) else { continue }
        if first == '#' {
            continue
        }

        // Table header [section]
        if first == '[' {
            if line.starts(with: "[[") {
                throw TomlParseError("array of tables [[...]] not supported", lineNum)
            }

            guard let some endPos = findUnquotedChar(line, ']') else {
                throw TomlParseError("unterminated table header", lineNum)
            }

            currentTable = line.asSlice().subslice(from: 1, to: endPos).trimmed().toOwned();
            ensureTable(root, currentTable);
            continue
        }

        let (key, value) = try parseKeyValue(line, lineNum);

        if currentTable.isEmpty {
            root.insert(key, value);
        } else {
            insertIntoTable(root, currentTable, key, value);
        }
    }

    .Ok(Value.Obj(root))
}

// ============================================================================
// KEY-VALUE PARSING
// ============================================================================

/// Splits a line on the first unquoted `=` and parses key + value.
func parseKeyValue(line: String, lineNum: Int64) -> (String, Value) throws TomlParseError {
    guard let some eqPos = findUnquotedChar(line, '=') else {
        throw TomlParseError("expected '=' in key-value pair", lineNum)
    }

    let lineSlice = line.asSlice();
    let rawKey = lineSlice.subslice(from: lineSlice.start, to: eqPos).trimmed().toOwned();
    let rawValue = lineSlice.subslice(from: eqPos + 1, to: lineSlice.end).trimmed().toOwned();

    let value = try parseTomlValue(stripInlineComment(rawValue), lineNum);
    .Ok((parseKey(rawKey), value))
}

/// Strips surrounding quotes from a key if present; returns bare keys unchanged.
func parseKey(s: String) -> String {
    if s.bytes.count >= 2 and s.starts(with: "\"") and s.ends(with: "\"") {
        return s.asSlice().subslice(from: 1, to: s.bytes.count - 1).toOwned()
    }
    s
}

/// Finds the byte offset of `target` outside double-quoted regions.
///
/// Walks characters but reports byte offsets, so the result can be handed
/// straight to `subslice(from:to:)`.
func findUnquotedChar(s: String, target: Char) -> Int64? {
    var offset: Int64 = 0;
    var inQuote = false;
    var escaped = false;

    for c in s.chars {
        if escaped {
            escaped = false
        } else if inQuote and c == '\\' {
            escaped = true
        } else if c == '"' {
            inQuote = not inQuote
        } else if not inQuote and c == target {
            return .Some(offset)
        }
        offset = offset + c.utf8Length()
    }

    .None
}

/// Strips an inline comment (`# ...`) from a value string, respecting quotes.
func stripInlineComment(s: String) -> String {
    guard let some pos = findUnquotedChar(s, '#') else { return s }
    s.asSlice().subslice(from: 0, to: pos).trimmed().toOwned()
}

// ============================================================================
// VALUE PARSING
// ============================================================================

/// Dispatches a trimmed value string to the appropriate sub-parser.
func parseTomlValue(s: String, lineNum: Int64) -> Value throws TomlParseError {
    guard let some first = s.chars(checked: 0) else {
        throw TomlParseError("empty value", lineNum)
    }

    // Booleans are whole-string tokens rather than a first-character form, so
    // they are matched before the character dispatch. (Nesting this as an inner
    // `match s` inside the `_` arm also trips an OSSA "consumed more than once"
    // ICE on the in-tree compiler.)
    if s == "true" {
        return .Ok(Value.Boolean(true))
    }
    if s == "false" {
        return .Ok(Value.Boolean(false))
    }

    match first {
        '"' => .Ok(Value.Str(try parseTomlString(s, lineNum))),
        '[' => .Ok(try parseTomlArray(s, lineNum)),
        '{' => .Ok(try parseInlineTable(s, lineNum)),
        _ => .Ok(try parseTomlNumber(s, lineNum))
    }
}

/// Parses a basic quoted TOML string, processing escape sequences.
func parseTomlString(s: String, lineNum: Int64) -> String throws TomlParseError {
    if s.bytes.count < 2 or not s.ends(with: "\"") {
        throw TomlParseError("unterminated string", lineNum)
    }

    let body = s.asSlice().subslice(from: 1, to: s.bytes.count - 1).toOwned();
    var result = String();
    var escaped = false;

    for c in body.chars {
        if escaped {
            result.append(char: try unescape(c, lineNum));
            escaped = false;
            continue
        }
        if c == '\\' {
            escaped = true;
            continue
        }
        result.append(char: c)
    }

    // A trailing backslash consumed the closing quote's predecessor and never paired up.
    if escaped {
        throw TomlParseError("unterminated escape in string", lineNum)
    }

    .Ok(result)
}

/// Maps the character after a backslash to the character it denotes.
func unescape(c: Char, lineNum: Int64) -> Char throws TomlParseError {
    match c {
        '"' => .Ok('"'),
        '\\' => .Ok('\\'),
        'n' => .Ok('\n'),
        't' => .Ok('\t'),
        'r' => .Ok('\r'),
        'b' => .Ok('\u{08}'),
        _ => throw TomlParseError("invalid escape sequence", lineNum)
    }
}

/// Parses a TOML number, dispatching to `Int64`/`Float64` parsing.
func parseTomlNumber(s: String, lineNum: Int64) -> Value throws TomlParseError {
    // TOML allows `_` as a digit separator; the stdlib parsers do not.
    let digits = s.replaced("_", with: "");

    if containsFloatMarker(s) or isNonFiniteToken(s) {
        guard let some f = Float64(parsing: digits) else {
            throw TomlParseError("invalid float: " + s, lineNum)
        }
        return .Ok(Value.Float(f))
    }

    guard let some n = Int64(parsing: digits) else {
        throw TomlParseError("invalid integer: " + s, lineNum)
    }
    .Ok(Value.Int(n))
}

/// Returns `true` if the string contains `.`, `e`, or `E` (float indicators).
///
/// Shared with the emitter, which uses it to decide whether a rendered float
/// needs a trailing `.0` to avoid re-reading as an integer.
public func containsFloatMarker(s: String) -> Bool {
    s.contains(where: { (c) in c == '.' or c == 'e' or c == 'E' })
}

/// Returns `true` for TOML's non-finite float tokens, with an optional sign.
///
/// These carry no `.`/`e` marker, so they need their own test to reach the
/// float parser.
func isNonFiniteToken(s: String) -> Bool {
    s == "inf" or s == "+inf" or s == "-inf" or s == "nan" or s == "+nan" or s == "-nan"
}

/// Parses a TOML inline array (`[value, ...]`).
func parseTomlArray(s: String, lineNum: Int64) -> Value throws TomlParseError {
    if not s.ends(with: "]") {
        throw TomlParseError("unterminated array", lineNum)
    }

    let inner = s.asSlice().subslice(from: 1, to: s.bytes.count - 1).trimmed().toOwned();
    var items: [Value] = [];

    for part in splitTomlItems(inner) {
        let item = part.trimmed().toOwned();
        if item.isEmpty { continue }
        items.append(try parseTomlValue(item, lineNum))
    }

    .Ok(Value.Arr(items))
}

/// Parses an inline table: `{ key = value, key2 = value2 }`
func parseInlineTable(s: String, lineNum: Int64) -> Value throws TomlParseError {
    if not s.ends(with: "}") {
        throw TomlParseError("unterminated inline table", lineNum)
    }

    let inner = s.asSlice().subslice(from: 1, to: s.bytes.count - 1).trimmed().toOwned();
    var obj: [String: Value] = [:];

    for part in splitTomlItems(inner) {
        let entry = part.trimmed().toOwned();
        if entry.isEmpty { continue }
        let (key, value) = try parseKeyValue(entry, lineNum);
        obj.insert(key, value);
    }

    .Ok(Value.Obj(obj))
}

/// Splits array or inline-table contents by commas, respecting quotes and nesting.
func splitTomlItems(s: String) -> [String] {
    var parts: [String] = [];
    var current = String();
    var depth: Int64 = 0;
    var inQuote = false;
    var escaped = false;

    for c in s.chars {
        if escaped {
            escaped = false
        } else if inQuote and c == '\\' {
            escaped = true
        } else if c == '"' {
            inQuote = not inQuote
        } else if not inQuote {
            if c == '[' or c == '{' {
                depth = depth + 1
            } else if c == ']' or c == '}' {
                depth = depth - 1
            } else if c == ',' and depth == 0 {
                parts.append(current);
                current = String();
                continue
            }
        }
        current.append(char: c)
    }

    if not current.isEmpty {
        parts.append(current)
    }

    parts
}

// ============================================================================
// TABLE MANAGEMENT
// ============================================================================

/// Creates the named table in `root` if it doesn't already exist.
func ensureTable(mutating root: [String: Value], name: String) {
    if root.contains(name) { return; }
    root.insert(name, Value.Obj([:]));
}

/// Inserts a key-value pair into the named sub-table within `root`.
func insertIntoTable(mutating root: [String: Value], table: String, key: String, value: Value) {
    // A non-Obj value at `table` is overwritten — TOML forbids reusing a key as a table.
    var obj: [String: Value] = match root(table) {
        some .Obj(existing) => existing,
        _ => [:]
    };
    obj.insert(key, value);
    root.insert(table, Value.Obj(obj));
}
