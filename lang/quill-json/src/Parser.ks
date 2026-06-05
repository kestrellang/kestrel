/// Recursive-descent JSON parser.
///
/// Converts a JSON string into a quill `Value` tree. The parser operates
/// in a single pass with no backtracking, indexed by byte offset for O(1)
/// slicing. String contents are decoded to UTF-8 `Char` only when escape
/// processing is needed; runs of unescaped bytes are copied verbatim.
///
/// # Examples
///
/// ```
/// import quill.json.parser.(parseJson)
///
/// let v = try parseJson("{\"a\": [1, true, null]}");
/// // v == Value.Obj([("a", Value.Arr([Value.Int(1), Value.Boolean(true), Value.Null]))])
/// ```

module quill.json.parser

import quill.value.(Value)
import quill.json.error.(JsonParseError)
import std.text.(decodeUtf8)

// ============================================================================
// JSON CURSOR
// ============================================================================

/// Mutable cursor tracking the current byte position in a JSON source string.
///
/// Indexes by byte offset so that substring extraction is O(1). Exposes
/// `Char` through `peekChar()` and `advanceChar()` for structural dispatch
/// — JSON's grammar tokens are ASCII, but string contents and `\uXXXX`
/// escapes require full Unicode decoding.
///
/// # Representation
///
/// Three fields: `source` (the full input string, retained for slicing),
/// `pos` (current byte offset), and `len` (cached `source.byteCount`).
struct JsonCursor: Cloneable {
    var source: String
    var pos: Int64
    var len: Int64

    /// @name Default
    /// Creates a cursor at the beginning of the given source string.
    init(source: String) {
        self.source = source;
        self.pos = 0;
        self.len = source.byteCount;
    }

    /// Returns a deep copy of the cursor (clones the source string).
    func clone() -> JsonCursor {
        JsonCursor(self.source.clone())
    }

    /// Returns `true` when the cursor has reached or passed the end of input.
    func atEnd() -> Bool {
        self.pos >= self.len
    }

    /// Returns the current code point and its byte width, or `.None` at end
    /// of input. Does **not** advance the cursor.
    ///
    /// Decodes UTF-8 directly from the source byte buffer at `self.pos`,
    /// avoiding the O(N) copy that `substringBytes` would incur.
    func peekChar() -> Optional[(Char, Int64)] {
        if self.pos >= self.len {
            return .None
        }
        match decodeUtf8(self.source.bytes.asRaw(), self.len, at: self.pos) {
            .Some(decoded) => .Some((decoded.char, decoded.bytesConsumed)),
            .None => .None
        }
    }

    /// Decodes the next code point, advances past it, and returns it.
    ///
    /// Returns an error if the cursor is already at end of input.
    mutating func advanceChar() -> Result[Char, JsonParseError] {
        match self.peekChar() {
            .Some(pair) => {
                self.pos = self.pos + pair.1;
                .Ok(pair.0)
            },
            .None => .Err(JsonParseError("unexpected end of input", self.pos))
        }
    }

    /// Skips ASCII whitespace (space, tab, newline, carriage return).
    mutating func skipWhitespace() {
        while let .Some(pair) = self.peekChar() {
            let c = pair.0;
            if c == ' ' or c == '\t' or c == '\n' or c == '\r' {
                self.pos = self.pos + pair.1
            } else {
                return
            }
        }
    }

    /// Advances one code point and returns an error if it doesn't match `c`.
    mutating func expect(c: Char) -> Result[(), JsonParseError] {
        let actual = try self.advanceChar();
        if actual == c {
            .Ok(())
        } else {
            var expected = String();
            expected.append(char: c);
            var got = String();
            got.append(char: actual);
            .Err(JsonParseError("expected '" + expected + "', got '" + got + "'", self.pos - 1))
        }
    }

    /// Advances past the exact bytes of `expected`, or errors if they don't match.
    ///
    /// Compares bytes directly at the cursor offset without copying the tail
    /// of the source string.
    mutating func expectStr(expected: String) -> Result[(), JsonParseError] {
        let startPos = self.pos;
        let expectedLen = expected.byteCount;
        if self.len - self.pos < expectedLen {
            return .Err(JsonParseError("expected '" + expected + "'", startPos))
        }
        let srcBytes = self.source.bytes;
        let expBytes = expected.bytes;
        var i: Int64 = 0;
        while i < expectedLen {
            if srcBytes(unchecked: self.pos + i) != expBytes(unchecked: i) {
                return .Err(JsonParseError("expected '" + expected + "'", startPos))
            }
            i = i + 1
        }
        self.pos = self.pos + expectedLen;
        .Ok(())
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Parses a complete JSON document into a `Value`.
///
/// Accepts any single JSON value (object, array, string, number, boolean,
/// or null). Leading and trailing whitespace is skipped; trailing
/// non-whitespace after the value produces an error.
///
/// # Examples
///
/// ```
/// let v = try parseJson("[1, 2, 3]");
/// // v == Value.Arr([Value.Int(1), Value.Int(2), Value.Int(3)])
/// ```
///
/// # Errors
///
/// Returns `JsonParseError` for any syntax violation, with a byte offset
/// pointing at the problem character.
public func parseJson(source: String) -> Result[Value, JsonParseError] {
    var cursor = JsonCursor(source);
    cursor.skipWhitespace();
    let value = try parseValue(cursor);
    cursor.skipWhitespace();
    if not cursor.atEnd() {
        return .Err(JsonParseError("unexpected trailing content", cursor.pos))
    }
    .Ok(value)
}

// ============================================================================
// INTERNAL PARSERS
// ============================================================================

/// Dispatches to the appropriate sub-parser based on the next character.
func parseValue(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    match cursor.peekChar() {
        .Some(pair) => {
            let c = pair.0;
            if c == 'n' {
                parseNull(cursor)
            } else if c == 't' {
                parseTrue(cursor)
            } else if c == 'f' {
                parseFalse(cursor)
            } else if c == '"' {
                parseJsonString(cursor)
            } else if c == '[' {
                parseArray(cursor)
            } else if c == '{' {
                parseObject(cursor)
            } else if c == '-' or c.isAsciiDigit {
                parseNumber(cursor)
            } else {
                var got = String();
                got.append(char: c);
                .Err(JsonParseError("unexpected character '" + got + "'", cursor.pos))
            }
        },
        .None => .Err(JsonParseError("unexpected end of input", cursor.pos))
    }
}

/// Consumes the literal `null`.
func parseNull(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expectStr("null");
    .Ok(Value.Null)
}

/// Consumes the literal `true`.
func parseTrue(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expectStr("true");
    .Ok(Value.Boolean(true))
}

/// Consumes the literal `false`.
func parseFalse(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expectStr("false");
    .Ok(Value.Boolean(false))
}

/// Parses a JSON number (integer or float) per RFC 8259 §6.
func parseNumber(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    let start = cursor.pos;
    var isFloat = false;

    // Optional minus sign
    if let .Some(pair) = cursor.peekChar() {
        if pair.0 == '-' {
            cursor.pos = cursor.pos + pair.1
        }
    }

    // Integer part
    match cursor.peekChar() {
        .Some(pair) => {
            let c = pair.0;
            if c == '0' {
                cursor.pos = cursor.pos + pair.1
            } else if c.isAsciiDigit {
                cursor.pos = cursor.pos + pair.1;
                while let .Some(p) = cursor.peekChar() {
                    if p.0.isAsciiDigit {
                        cursor.pos = cursor.pos + p.1
                    } else {
                        break
                    }
                }
            } else {
                return .Err(JsonParseError("invalid number", start))
            }
        },
        .None => return .Err(JsonParseError("unexpected end of input in number", start))
    }

    // Fractional part
    if let .Some(pair) = cursor.peekChar() {
        if pair.0 == '.' {
            isFloat = true;
            cursor.pos = cursor.pos + pair.1;
            var hasDigit = false;
            while let .Some(p) = cursor.peekChar() {
                if p.0.isAsciiDigit {
                    cursor.pos = cursor.pos + p.1;
                    hasDigit = true
                } else {
                    break
                }
            }
            if not hasDigit {
                return .Err(JsonParseError("expected digit after '.'", cursor.pos))
            }
        }
    }

    // Exponent
    if let .Some(pair) = cursor.peekChar() {
        let c = pair.0;
        if c == 'e' or c == 'E' {
            isFloat = true;
            cursor.pos = cursor.pos + pair.1;
            // Optional sign
            if let .Some(p) = cursor.peekChar() {
                if p.0 == '+' or p.0 == '-' {
                    cursor.pos = cursor.pos + p.1
                }
            }
            var hasDigit = false;
            while let .Some(p) = cursor.peekChar() {
                if p.0.isAsciiDigit {
                    cursor.pos = cursor.pos + p.1;
                    hasDigit = true
                } else {
                    break
                }
            }
            if not hasDigit {
                return .Err(JsonParseError("expected digit in exponent", cursor.pos))
            }
        }
    }

    let numStr = cursor.source.asSlice().subslice(from: start, to: cursor.pos).toOwned();

    if isFloat {
        match parseFloat64(numStr) {
            .Some(f) => .Ok(Value.Float(f)),
            .None => .Err(JsonParseError("invalid float: " + numStr, start))
        }
    } else {
        match parseInt64(numStr) {
            .Some(n) => .Ok(Value.Int(n)),
            .None => .Err(JsonParseError("invalid integer: " + numStr, start))
        }
    }
}

/// Parses a JSON string and wraps it in `Value.Str`.
func parseJsonString(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    let s = try parseRawString(cursor);
    .Ok(Value.Str(s))
}

/// Parses a JSON string literal and returns the unescaped content.
///
/// Uses a fast path that copies runs of non-escape bytes verbatim (no
/// UTF-8 decode/re-encode overhead). Falls back to per-character decoding
/// only for `\` escape sequences, where `\uXXXX` may produce multi-byte
/// code points.
func parseRawString(mutating cursor: JsonCursor) -> Result[String, JsonParseError] {
    try cursor.expect('"');
    var result = String();
    let srcBytes = cursor.source.bytes;

    while true {
        // Fast path: scan a run of plain bytes (no '"', no '\').
        let runStart = cursor.pos;
        while cursor.pos < cursor.len {
            let b = srcBytes(unchecked: cursor.pos);
            if b == 34 or b == 92 {
                break
            }
            cursor.pos = cursor.pos + 1
        }

        // Copy the run verbatim.
        result.append(srcBytes.substring(runStart..<cursor.pos));

        if cursor.pos >= cursor.len {
            return .Err(JsonParseError("unterminated string", cursor.pos))
        }

        let terminator = srcBytes(unchecked: cursor.pos);
        if terminator == 34 {
            cursor.pos = cursor.pos + 1;
            return .Ok(result)
        }

        // Backslash escape. Step past `\` and decode the next char.
        cursor.pos = cursor.pos + 1;
        let esc = try cursor.advanceChar();
        // \" \\ \/ \b \f \n \r \t \uXXXX
        if esc == '"' {
            result.append(char: '"')
        } else if esc == '\\' {
            result.append(char: '\\')
        } else if esc == '/' {
            result.append(char: '/')
        } else if esc == 'b' {
            result.append(char: Char(8).unwrap())
        } else if esc == 'f' {
            result.append(char: Char(12).unwrap())
        } else if esc == 'n' {
            result.append(char: '\n')
        } else if esc == 'r' {
            result.append(char: '\r')
        } else if esc == 't' {
            result.append(char: '\t')
        } else if esc == 'u' {
            let codepoint = try parseUnicodeEscape(cursor);
            result.append(char: Char(UInt32(from: codepoint)).unwrap())
        } else {
            return .Err(JsonParseError("invalid escape sequence", cursor.pos - 1))
        }
    }

    // Unreachable, but needed for type checker
    .Err(JsonParseError("unterminated string", cursor.pos))
}

/// Parses a 4-digit hex escape `\uXXXX` and returns the code point.
func parseUnicodeEscape(mutating cursor: JsonCursor) -> Result[Int64, JsonParseError] {
    var value: Int64 = 0;
    var i: Int64 = 0;
    while i < 4 {
        let c = try cursor.advanceChar();
        let digit = hexDigitValue(c);
        if digit < 0 {
            return .Err(JsonParseError("invalid hex digit in unicode escape", cursor.pos - 1))
        }
        value = value * 16 + digit;
        i = i + 1
    }
    .Ok(value)
}

/// Returns the numeric value of a hex digit, or -1 if not a hex digit.
func hexDigitValue(c: Char) -> Int64 {
    if let .Some(d) = c.digitValue() {
        return Int64(from: d)
    }
    if c >= 'A' and c <= 'F' {
        return Int64(from: c.value()) - 55
    }
    if c >= 'a' and c <= 'f' {
        return Int64(from: c.value()) - 87
    }
    0 - 1
}

/// Parses a JSON array (`[value, ...]`).
func parseArray(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expect('[');
    cursor.skipWhitespace();

    var items = Array[Value]();

    // Empty array shortcut
    match cursor.peekChar() {
        .Some(pair) => {
            if pair.0 == ']' {
                cursor.pos = cursor.pos + pair.1;
                return .Ok(Value.Arr(items))
            }
        },
        .None => return .Err(JsonParseError("unexpected end of input in array", cursor.pos))
    }

    let first = try parseValue(cursor);
    items.append(first);

    while true {
        cursor.skipWhitespace();
        match cursor.peekChar() {
            .Some(pair) => {
                let c = pair.0;
                if c == ']' {
                    cursor.pos = cursor.pos + pair.1;
                    return .Ok(Value.Arr(items))
                }
                if c == ',' {
                    cursor.pos = cursor.pos + pair.1;
                    cursor.skipWhitespace();
                    let item = try parseValue(cursor);
                    items.append(item)
                } else {
                    return .Err(JsonParseError("expected ',' or ']' in array", cursor.pos))
                }
            },
            .None => return .Err(JsonParseError("unexpected end of input in array", cursor.pos))
        }
    }

    .Err(JsonParseError("unexpected end of input in array", cursor.pos))
}

/// Parses a JSON object (`{"key": value, ...}`).
func parseObject(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expect('{');
    cursor.skipWhitespace();

    var obj = Dictionary[String, Value]();

    // Empty object shortcut
    match cursor.peekChar() {
        .Some(pair) => {
            if pair.0 == '}' {
                cursor.pos = cursor.pos + pair.1;
                return .Ok(Value.Obj(obj))
            }
        },
        .None => return .Err(JsonParseError("unexpected end of input in object", cursor.pos))
    }

    let firstKey = try parseRawString(cursor);
    cursor.skipWhitespace();
    try cursor.expect(':');
    cursor.skipWhitespace();
    let firstVal = try parseValue(cursor);
     obj.insert(firstKey, firstVal);

    while true {
        cursor.skipWhitespace();
        match cursor.peekChar() {
            .Some(pair) => {
                let c = pair.0;
                if c == '}' {
                    cursor.pos = cursor.pos + pair.1;
                    return .Ok(Value.Obj(obj))
                }
                if c == ',' {
                    cursor.pos = cursor.pos + pair.1;
                    cursor.skipWhitespace();
                    let key = try parseRawString(cursor);
                    cursor.skipWhitespace();
                    try cursor.expect(':');
                    cursor.skipWhitespace();
                    let val = try parseValue(cursor);
                     obj.insert(key, val);
                } else {
                    return .Err(JsonParseError("expected ',' or '}' in object", cursor.pos))
                }
            },
            .None => return .Err(JsonParseError("unexpected end of input in object", cursor.pos))
        }
    }

    .Err(JsonParseError("unexpected end of input in object", cursor.pos))
}

// ============================================================================
// NUMBER PARSING HELPERS
// ============================================================================

/// Parses a string as an Int64. Returns `.None` on failure.
func parseInt64(s: String) -> Optional[Int64] {
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
    }

    var result: Int64 = 0;
    var current: Optional[Char] = .Some(first);
    while let .Some(c) = current {
        match c.digitValue() {
            .Some(d) => result = result * 10 + Int64(from: d),
            .None => return .None
        }
        current = iter.next()
    }

    if negative { .Some(0 - result) } else { .Some(result) }
}

/// Parses a string as a Float64 — integer part, fractional part, exponent.
func parseFloat64(s: String) -> Optional[Float64] {
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
