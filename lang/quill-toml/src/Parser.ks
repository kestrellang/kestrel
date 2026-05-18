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
// TOML CURSOR
// ============================================================================

/// Mutable cursor tracking the current byte position and line number in a
/// TOML source string.
///
/// Unlike the JSON parser (which is character-oriented), this parser works
/// line-by-line: `nextLine()` extracts one logical line at a time, handling
/// `\n`, `\r\n`, and `\r` line endings.
///
/// # Representation
///
/// Four fields: `source` (the full input), `pos` (current byte offset),
/// `len` (cached `source.byteCount`), and `line` (1-based line counter).
struct TomlCursor: Cloneable {
    var source: String
    var pos: Int64
    var len: Int64
    var line: Int64

    /// @name Default
    /// Creates a cursor at the beginning of the given source string.
    init(source: String) {
        self.source = source;
        self.pos = 0;
        self.len = source.byteCount;
        self.line = 1;
    }

    /// Returns `true` when the cursor has reached or passed the end of input.
    func atEnd() -> Bool {
        self.pos >= self.len
    }

    /// Extracts the next logical line and its 1-based line number.
    ///
    /// Recognizes `\n`, `\r\n`, and bare `\r` as line terminators. Returns
    /// `.None` when the cursor is at end of input.
    mutating func nextLine() -> Optional[(String, Int64)] {
        if self.atEnd() {
            return .None
        }

        let start = self.pos;
        let lineNum = self.line;
        let bytes = self.source.bytes;
        let slice = self.source.asSlice();

        while self.pos < self.len {
            let b = bytes(unchecked: self.pos);
            if b == 10 {
                let line = slice.subslice(from: start, to: self.pos).toOwned();
                self.pos = self.pos + 1;
                self.line = self.line + 1;
                return .Some((line, lineNum))
            }
            if b == 13 {
                let line = slice.subslice(from: start, to: self.pos).toOwned();
                self.pos = self.pos + 1;
                if self.pos < self.len and bytes(unchecked: self.pos) == 10 {
                    self.pos = self.pos + 1
                }
                self.line = self.line + 1;
                return .Some((line, lineNum))
            }
            self.pos = self.pos + 1
        }

        .Some((slice.subslice(from: start, to: self.len).toOwned(), lineNum))
    }

    /// Returns a copy of this cursor with the same position and state.
    func clone() -> TomlCursor {
        var c = TomlCursor(self.source.clone());
        c.pos = self.pos;
        c.len = self.len;
        c.line = self.line;
        c
    }
}

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
/// Returns `TomlParseError` for syntax violations, with a line number
/// pointing at the problem line.
public func parseToml(source: String) -> Result[Value, TomlParseError] {
    var root = Dictionary[String, Value]();
    var currentTable = "";
    var cursor = TomlCursor(source);

    while let .Some(pair) = cursor.nextLine() {
        let lineNum = pair.1;
        let line = pair.0.trimmed().toOwned();

        // Skip empty lines and comments
        if line.isEmpty or line.starts(with: "#") {
            continue
        }

        // Table header [section]
        if line.starts(with: "[") {
            if line.starts(with: "[[") {
                return .Err(TomlParseError("array of tables [[...]] not supported", lineNum))
            }

            match findUnquotedByte(line, 93) {
                .Some(endPos) => {
                    currentTable = line.asSlice().subslice(from: 1, to: endPos).trimmed().toOwned();
                    ensureTable(root, currentTable);
                    continue
                },
                .None => return .Err(TomlParseError("unterminated table header", lineNum))
            }
        }

        let parsed = try parseKeyValue(line, lineNum);
        let key = parsed.0;
        let val = parsed.1;

        if currentTable.isEmpty {
            let _ = root.insert(key, val);
        } else {
            insertIntoTable(root, currentTable, key, val)
        }
    }

    .Ok(Value.Obj(root))
}

// ============================================================================
// KEY-VALUE PARSING
// ============================================================================

/// Splits a line on the first unquoted `=` and parses key + value.
func parseKeyValue(line: String, lineNum: Int64) -> Result[(String, Value), TomlParseError] {
    let eqPos = match findUnquotedByte(line, 61) {
        .Some(p) => p,
        .None => return .Err(TomlParseError("expected '=' in key-value pair", lineNum))
    };

    let lineSlice = line.asSlice();
    let rawKey = lineSlice.subslice(from: lineSlice.start, to: eqPos).trimmed().toOwned();
    let rawVal = lineSlice.subslice(from: eqPos + 1, to: lineSlice.end).trimmed().toOwned();
    let valStr = stripInlineComment(rawVal);

    let key = parseKey(rawKey);
    let value = try parseTomlValue(valStr, lineNum);

    .Ok((key, value))
}

/// Strips surrounding quotes from a key if present; returns bare keys unchanged.
func parseKey(s: String) -> String {
    if s.byteCount >= 2 and s.starts(with: "\"") and s.ends(with: "\"") {
        return s.asSlice().subslice(from: 1, to: s.byteCount - 1).toOwned()
    }
    s
}

/// Finds the byte offset of `target` outside double-quoted regions.
///
/// Tracks quote/escape state so that the byte 34 (`"`) inside a quoted
/// string is not confused with the target.
func findUnquotedByte(s: String, target: UInt8) -> Optional[Int64] {
    let bytes = s.bytes;
    let len = s.byteCount;
    var inQuote = false;
    var escaped = false;
    var i: Int64 = 0;

    while i < len {
        let b = bytes(unchecked: i);
        if escaped {
            escaped = false
        } else if inQuote and b == 92 {
            escaped = true
        } else if b == 34 {
            inQuote = not inQuote
        } else if not inQuote and b == target {
            return .Some(i)
        }
        i = i + 1
    }

    .None
}

/// Strips an inline comment (`# ...`) from a value string, respecting quotes.
func stripInlineComment(s: String) -> String {
    match findUnquotedByte(s, 35) {
        .Some(pos) => s.asSlice().subslice(from: 0, to: pos).trimmed().toOwned(),
        .None => s
    }
}

// ============================================================================
// VALUE PARSING
// ============================================================================

/// Dispatches a trimmed value string to the appropriate sub-parser.
func parseTomlValue(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    if s.isEmpty {
        return .Err(TomlParseError("empty value", lineNum))
    }

    if s.starts(with: "\"") {
        let str = try parseTomlString(s, lineNum);
        return .Ok(Value.Str(str))
    }

    if s.starts(with: "[") {
        return parseTomlArray(s, lineNum)
    }

    if s.starts(with: "{") {
        return parseInlineTable(s, lineNum)
    }

    if s == "true" {
        return .Ok(Value.Boolean(true))
    }
    if s == "false" {
        return .Ok(Value.Boolean(false))
    }

    parseTomlNumber(s, lineNum)
}

/// Parses a basic quoted TOML string, processing escape sequences.
func parseTomlString(s: String, lineNum: Int64) -> Result[String, TomlParseError] {
    if s.byteCount < 2 or not s.ends(with: "\"") {
        return .Err(TomlParseError("unterminated string", lineNum))
    }

    var result = String();
    let slice = s.asSlice();
    let bytes = s.bytes;
    var i: Int64 = 1;
    let end = s.byteCount - 1;
    var segStart: Int64 = 1;

    while i < end {
        let b = bytes(unchecked: i);
        if b == 92 {
            // flush plain segment before escape
            if segStart < i {
                result.append(slice.subslice(from: segStart, to: i).toOwned())
            }
            i = i + 1;
            if i >= end {
                return .Err(TomlParseError("unterminated escape in string", lineNum))
            }

            let esc = bytes(unchecked: i);
            if esc == 34 {
                result.append(char: '"')
            } else if esc == 92 {
                result.append(char: '\\')
            } else if esc == 110 {
                result.append(char: '\n')
            } else if esc == 116 {
                result.append(char: '\t')
            } else if esc == 114 {
                result.append(char: '\r')
            } else {
                return .Err(TomlParseError("invalid escape sequence", lineNum))
            }
            i = i + 1;
            segStart = i
        } else {
            i = i + 1
        }
    }

    if segStart < end {
        result.append(slice.subslice(from: segStart, to: end).toOwned())
    }

    .Ok(result)
}

/// Parses a TOML number — dispatches to int or float based on `.`/`e`/`E`.
func parseTomlNumber(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    let isFloat = containsFloatMarker(s);

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

/// Returns `true` if the string contains `.`, `e`, or `E` (float indicators).
func containsFloatMarker(s: String) -> Bool {
    let bytes = s.bytes;
    var i: Int64 = 0;
    while i < s.byteCount {
        let b = bytes(unchecked: i);
        if b == 46 or b == 101 or b == 69 {
            return true
        }
        i = i + 1
    }
    false
}

/// Parses a TOML inline array (`[value, ...]`).
func parseTomlArray(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    if not s.ends(with: "]") {
        return .Err(TomlParseError("unterminated array", lineNum))
    }

    let inner = s.asSlice().subslice(from: 1, to: s.byteCount - 1).trimmed().toOwned();
    if inner.isEmpty {
        return .Ok(Value.Arr(Array[Value]()))
    }

    var items = Array[Value]();
    var parts = splitTomlItems(inner);
    var pi: Int64 = 0;
    while pi < parts.count {
        let part = parts(unchecked: pi).trimmed().toOwned();
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

    let inner = s.asSlice().subslice(from: 1, to: s.byteCount - 1).trimmed().toOwned();
    if inner.isEmpty {
        return .Ok(Value.Obj(Dictionary[String, Value]()))
    }

    var obj = Dictionary[String, Value]();
    var parts = splitTomlItems(inner);
    var pi: Int64 = 0;
    while pi < parts.count {
        let part = parts(unchecked: pi).trimmed().toOwned();
        if not part.isEmpty {
            let kv = try parseKeyValue(part, lineNum);
            let _ = obj.insert(kv.0, kv.1);
        }
        pi = pi + 1
    }

    .Ok(Value.Obj(obj))
}

/// Splits array or inline-table contents by commas, respecting quotes and nesting.
func splitTomlItems(s: String) -> Array[String] {
    var parts = Array[String]();
    var depth: Int64 = 0;
    var inQuote = false;
    var escaped = false;
    var start: Int64 = 0;
    var i: Int64 = 0;
    let bytes = s.bytes;
    let len = s.byteCount;
    let slice = s.asSlice();

    while i < len {
        let b = bytes(unchecked: i);
        if escaped {
            escaped = false
        } else if inQuote and b == 92 {
            escaped = true
        } else if b == 34 {
            inQuote = not inQuote
        } else if not inQuote {
            if b == 91 or b == 123 {
                depth = depth + 1
            } else if b == 93 or b == 125 {
                depth = depth - 1
            } else if b == 44 and depth == 0 {
                parts.append(slice.subslice(from: start, to: i).toOwned());
                start = i + 1
            }
        }
        i = i + 1
    }

    if start < len {
        parts.append(slice.subslice(from: start, to: len).toOwned())
    }

    parts
}

// ============================================================================
// TABLE MANAGEMENT
// ============================================================================

/// Creates the named table in `root` if it doesn't already exist.
func ensureTable(mutating root: Dictionary[String, Value], name: String) {
    match root(name) {
        .Some(_) => {},
        .None => { let _ = root.insert(name, Value.Obj(Dictionary[String, Value]())); }
    }
}

/// Inserts a key-value pair into the named sub-table within `root`.
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
            // TOML allows underscore separators.
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

    var cleaned = String();
    var iter = s.chars.iter();
    while let .Some(c) = iter.next() {
        if c != '_' {
            cleaned.append(char: c)
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
