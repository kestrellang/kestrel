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
// TOML CURSOR
// ============================================================================

/// Tracks the current byte offset and source line while parsing TOML.
struct TomlCursor {
    var source: String
    var pos: Int64
    var len: Int64
    var line: Int64

    init(source: String) {
        self.source = source;
        self.pos = 0;
        self.len = source.byteCount;
        self.line = 1;
    }

    func atEnd() -> Bool {
        self.pos >= self.len
    }

    /// Returns the next logical line, recognizing `\n`, `\r\n`, and `\r`.
    mutating func nextLine() -> Optional[(String, Int64)] {
        if self.atEnd() {
            return .None
        }

        let start = self.pos;
        let lineNum = self.line;
        let bytes = self.source.bytes;

        while self.pos < self.len {
            let b = bytes(unchecked: self.pos);
            if b == 10 {
                let line = self.source.substringBytes(from: start, to: self.pos);
                self.pos = self.pos + 1;
                self.line = self.line + 1;
                return .Some((line, lineNum))
            }
            if b == 13 {
                let line = self.source.substringBytes(from: start, to: self.pos);
                self.pos = self.pos + 1;
                if self.pos < self.len and bytes(unchecked: self.pos) == 10 {
                    self.pos = self.pos + 1
                }
                self.line = self.line + 1;
                return .Some((line, lineNum))
            }
            self.pos = self.pos + 1
        }

        .Some((self.source.substringBytes(from: start, to: self.len), lineNum))
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Parses a TOML string into a Value.Object.
public func parseToml(source: String) -> Result[Value, TomlParseError] {
    var root = Dictionary[String, Value]();
    var currentTable = "";
    var cursor = TomlCursor(source);

    while let .Some(pair) = cursor.nextLine() {
        let lineNum = pair.1;
        let line = pair.0.trimmed();

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
                    currentTable = line.substringBytes(from: 1, to: endPos).trimmed();
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

func parseKeyValue(line: String, lineNum: Int64) -> Result[(String, Value), TomlParseError] {
    let eqPos = match findUnquotedByte(line, 61) {
        .Some(p) => p,
        .None => return .Err(TomlParseError("expected '=' in key-value pair", lineNum))
    };

    let rawKey = line.substringBytes(from: 0, to: eqPos).trimmed();
    let rawVal = line.substringBytes(from: eqPos + 1, to: line.byteCount).trimmed();
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

/// Finds an ASCII byte outside double-quoted strings.
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
            inQuote = inQuote == false
        } else if inQuote == false and b == target {
            return .Some(i)
        }
        i = i + 1
    }

    .None
}

/// Strips an inline comment (`# ...`) from a value string, respecting quotes.
func stripInlineComment(s: String) -> String {
    match findUnquotedByte(s, 35) {
        .Some(pos) => s.substringBytes(from: 0, to: pos).trimmed(),
        .None => s
    }
}

// ============================================================================
// VALUE PARSING
// ============================================================================

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

func parseTomlString(s: String, lineNum: Int64) -> Result[String, TomlParseError] {
    if s.byteCount < 2 or not s.ends(with: "\"") {
        return .Err(TomlParseError("unterminated string", lineNum))
    }

    var result = String();
    let bytes = s.bytes;
    var i: Int64 = 1;
    let end = s.byteCount - 1;

    while i < end {
        let b = bytes(unchecked: i);
        if b == 92 {
            i = i + 1;
            if i >= end {
                return .Err(TomlParseError("unterminated escape in string", lineNum))
            }

            let esc = bytes(unchecked: i);
            if esc == 34 {
                result.appendByte(34)
            } else if esc == 92 {
                result.appendByte(92)
            } else if esc == 110 {
                result.appendByte(10)
            } else if esc == 116 {
                result.appendByte(9)
            } else if esc == 114 {
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

func parseTomlArray(s: String, lineNum: Int64) -> Result[Value, TomlParseError] {
    if not s.ends(with: "]") {
        return .Err(TomlParseError("unterminated array", lineNum))
    }

    let inner = s.substringBytes(from: 1, to: s.byteCount - 1).trimmed();
    if inner.isEmpty {
        return .Ok(Value.Arr(Array[Value]()))
    }

    var items = Array[Value]();
    var parts = splitTomlItems(inner);
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
    var parts = splitTomlItems(inner);
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

    while i < len {
        let b = bytes(unchecked: i);
        if escaped {
            escaped = false
        } else if inQuote and b == 92 {
            escaped = true
        } else if b == 34 {
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
