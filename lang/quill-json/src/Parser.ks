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
/// `pos` (current byte offset), and `len` (cached `source.bytes.count`).
struct JsonCursor: Cloneable {
    var source: String
    var pos: Int64
    var len: Int64

    /// @name Default
    /// Creates a cursor at the beginning of the given source string.
    init(source: String) {
        self.source = source;
        self.pos = 0;
        self.len = source.bytes.count;
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
    func peekChar() -> (Char, Int64)? {
        if self.atEnd() {
            return .None
        }

        guard let some decoded = decodeUtf8(self.source.bytes.asRaw(), self.len, at: self.pos) else {
            return .None
        }
        .Some((decoded.char, decoded.bytesConsumed))
    }

    /// Decodes the next code point, advances past it, and returns it.
    ///
    /// Throws if the cursor is already at end of input.
    mutating func advanceChar() -> Char throws JsonParseError {
        guard let some (c, width) = self.peekChar() else {
            throw JsonParseError("unexpected end of input", self.pos)
        }
        self.pos = self.pos + width;
        .Ok(c)
    }

    /// Skips ASCII whitespace (space, tab, newline, carriage return).
    mutating func skipWhitespace() {
        while let some (c, width) = self.peekChar() {
            if c != ' ' and c != '\t' and c != '\n' and c != '\r' {
                return;
            }
            self.pos = self.pos + width
        }
    }

    /// Advances one code point and throws if it doesn't match `c`.
    mutating func expect(c: Char) -> () throws JsonParseError {
        let actual = try self.advanceChar();
        if actual == c {
            return .Ok(())
        }

        var expected = String();
        expected.append(char: c);
        var got = String();
        got.append(char: actual);
        throw JsonParseError("expected '" + expected + "', got '" + got + "'", self.pos - 1)
    }

    /// Advances past the exact bytes of `expected`, or throws if they don't match.
    ///
    /// Compares bytes directly at the cursor offset without copying the tail
    /// of the source string.
    mutating func expectStr(expected: String) -> () throws JsonParseError {
        let startPos = self.pos;
        let expectedLen = expected.bytes.count;
        if self.len - self.pos < expectedLen {
            throw JsonParseError("expected '" + expected + "'", startPos)
        }

        let srcBytes = self.source.bytes;
        for (offset, expectedByte) in expected.bytes.iter().enumerate() {
            if srcBytes(unchecked: self.pos + offset) != expectedByte {
                throw JsonParseError("expected '" + expected + "'", startPos)
            }
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
/// Throws `JsonParseError` for any syntax violation, with a byte offset
/// pointing at the problem character.
public func parseJson(source: String) -> Value throws JsonParseError {
    var cursor = JsonCursor(source);
    cursor.skipWhitespace();
    let value = try parseValue(cursor);
    cursor.skipWhitespace();

    if not cursor.atEnd() {
        throw JsonParseError("unexpected trailing content", cursor.pos)
    }
    .Ok(value)
}

// ============================================================================
// INTERNAL PARSERS
// ============================================================================

/// Dispatches to the appropriate sub-parser based on the next character.
func parseValue(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    guard let some (c, _) = cursor.peekChar() else {
        throw JsonParseError("unexpected end of input", cursor.pos)
    }

    // Numbers are the one production keyed on a character *class*, so they are
    // tested before the match. (A `digit if digit.isAsciiDigit` arm would be
    // the natural spelling, but pattern-guard arms currently miscompile — see
    // the OSSA note in `unexpectedCharacter`.)
    if c == '-' or c.isAsciiDigit {
        return parseNumber(cursor)
    }

    match c {
        'n' => parseNull(cursor),
        't' => parseTrue(cursor),
        'f' => parseFalse(cursor),
        '"' => parseJsonString(cursor),
        '[' => parseArray(cursor),
        '{' => parseObject(cursor),
        _ => unexpectedCharacter(c, at: cursor.pos)
    }
}

/// Builds the "unexpected character" error for the byte offset `at`.
///
/// NOTE: `match` arms carrying a pattern guard (`x if cond => ...`) currently
/// fail OSSA verification when the arms produce owned values, so dispatch that
/// needs a character *class* is written as an `if` before the match.
func unexpectedCharacter(c: Char, at offset: Int64) -> Value throws JsonParseError {
    var got = String();
    got.append(char: c);
    throw JsonParseError("unexpected character '" + got + "'", offset)
}

/// Consumes the literal `null`.
func parseNull(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    try cursor.expectStr("null");
    .Ok(Value.Null)
}

/// Consumes the literal `true`.
func parseTrue(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    try cursor.expectStr("true");
    .Ok(Value.Boolean(true))
}

/// Consumes the literal `false`.
func parseFalse(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    try cursor.expectStr("false");
    .Ok(Value.Boolean(false))
}

/// Parses a JSON number (integer or float) per RFC 8259 §6.
///
/// Validates the grammar while scanning, then hands the matched span to the
/// stdlib parsers — which also reject values too large for the target type.
func parseNumber(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    let start = cursor.pos;
    var isFloat = false;

    // Optional minus sign
    if let some (sign, signWidth) = cursor.peekChar() {
        if sign == '-' {
            cursor.pos = cursor.pos + signWidth
        }
    }

    // Integer part: a lone `0`, or a digit run with no leading zero.
    guard let some (first, firstWidth) = cursor.peekChar() else {
        throw JsonParseError("unexpected end of input in number", start)
    }

    if not first.isAsciiDigit {
        throw JsonParseError("invalid number", start)
    }

    cursor.pos = cursor.pos + firstWidth;
    // A leading `0` stands alone; any other digit starts a run.
    if first != '0' {
        skipDigits(cursor);
    }

    // Fractional part
    if let some (dot, dotWidth) = cursor.peekChar() {
        if dot == '.' {
            isFloat = true;
            cursor.pos = cursor.pos + dotWidth;
            if not skipDigits(cursor) {
                throw JsonParseError("expected digit after '.'", cursor.pos)
            }
        }
    }

    // Exponent
    if let some (marker, markerWidth) = cursor.peekChar() {
        if marker == 'e' or marker == 'E' {
            isFloat = true;
            cursor.pos = cursor.pos + markerWidth;

            if let some (sign, signWidth) = cursor.peekChar() {
                if sign == '+' or sign == '-' {
                    cursor.pos = cursor.pos + signWidth
                }
            }

            if not skipDigits(cursor) {
                throw JsonParseError("expected digit in exponent", cursor.pos)
            }
        }
    }

    let numStr = cursor.source.asSlice().subslice(from: start, to: cursor.pos).toOwned();

    if isFloat {
        guard let some f = Float64(parsing: numStr) else {
            throw JsonParseError("invalid float: " + numStr, start)
        }
        return .Ok(Value.Float(f))
    }

    guard let some n = Int64(parsing: numStr) else {
        throw JsonParseError("invalid integer: " + numStr, start)
    }
    .Ok(Value.Int(n))
}

/// Consumes a run of ASCII digits; reports whether at least one was consumed.
func skipDigits(mutating cursor: JsonCursor) -> Bool {
    var consumed = false;

    while let some (c, width) = cursor.peekChar() {
        if not c.isAsciiDigit {
            return consumed;
        }
        cursor.pos = cursor.pos + width;
        consumed = true
    }

    consumed
}

/// Parses a JSON string and wraps it in `Value.Str`.
func parseJsonString(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    let s = try parseRawString(cursor);
    .Ok(Value.Str(s))
}

/// Parses a JSON string literal and returns the unescaped content.
///
/// Uses a fast path that copies runs of non-escape bytes verbatim (no
/// UTF-8 decode/re-encode overhead). Falls back to per-character decoding
/// only for `\` escape sequences, where `\uXXXX` may produce multi-byte
/// code points.
func parseRawString(mutating cursor: JsonCursor) -> String throws JsonParseError {
    try cursor.expect('"');
    var result = String();
    let srcBytes = cursor.source.bytes;

    loop {
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
            throw JsonParseError("unterminated string", cursor.pos)
        }

        if srcBytes(unchecked: cursor.pos) == 34 {
            cursor.pos = cursor.pos + 1;
            return .Ok(result)
        }

        // Backslash escape. Step past `\` and decode the next char.
        cursor.pos = cursor.pos + 1;
        result.append(char: try unescape(cursor))
    }
}

/// Decodes one escape sequence, with the leading `\` already consumed.
func unescape(mutating cursor: JsonCursor) -> Char throws JsonParseError {
    let esc = try cursor.advanceChar();

    match esc {
        '"' => .Ok('"'),
        '\\' => .Ok('\\'),
        '/' => .Ok('/'),
        'b' => .Ok('\u{08}'),
        'f' => .Ok('\u{0C}'),
        'n' => .Ok('\n'),
        'r' => .Ok('\r'),
        't' => .Ok('\t'),
        'u' => {
            let codepoint = try parseUnicodeEscape(cursor);
            // Surrogate halves and out-of-range values have no scalar; reject
            // them rather than unwrapping a `.None`.
            guard let some decoded = Char(UInt32(from: codepoint)) else {
                throw JsonParseError("invalid unicode escape", cursor.pos - 6)
            }
            .Ok(decoded)
        },
        _ => throw JsonParseError("invalid escape sequence", cursor.pos - 1)
    }
}

/// Parses a 4-digit hex escape `\uXXXX` and returns the code point.
func parseUnicodeEscape(mutating cursor: JsonCursor) -> Int64 throws JsonParseError {
    var value: Int64 = 0;

    for _ in 0..<4 {
        let c = try cursor.advanceChar();
        guard let some digit = hexDigitValue(c) else {
            throw JsonParseError("invalid hex digit in unicode escape", cursor.pos - 1)
        }
        value = value * 16 + digit
    }

    .Ok(value)
}

/// Returns the numeric value of a hex digit, or `.None` if not a hex digit.
func hexDigitValue(c: Char) -> Int64? {
    if let some d = c.digitValue() {
        return .Some(Int64(from: d))
    }
    if c >= 'A' and c <= 'F' {
        return .Some(Int64(from: c.value()) - 55)
    }
    if c >= 'a' and c <= 'f' {
        return .Some(Int64(from: c.value()) - 87)
    }
    .None
}

/// Parses a JSON array (`[value, ...]`).
func parseArray(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    try cursor.expect('[');
    cursor.skipWhitespace();

    var items: [Value] = [];

    guard let some (first, firstWidth) = cursor.peekChar() else {
        throw JsonParseError("unexpected end of input in array", cursor.pos)
    }

    // Empty array shortcut
    if first == ']' {
        cursor.pos = cursor.pos + firstWidth;
        return .Ok(Value.Arr(items))
    }

    items.append(try parseValue(cursor));

    loop {
        cursor.skipWhitespace();

        guard let some (c, width) = cursor.peekChar() else {
            throw JsonParseError("unexpected end of input in array", cursor.pos)
        }

        if c == ']' {
            cursor.pos = cursor.pos + width;
            return .Ok(Value.Arr(items))
        }

        if c != ',' {
            throw JsonParseError("expected ',' or ']' in array", cursor.pos)
        }

        cursor.pos = cursor.pos + width;
        cursor.skipWhitespace();
        items.append(try parseValue(cursor))
    }
}

/// Parses a JSON object (`{"key": value, ...}`).
func parseObject(mutating cursor: JsonCursor) -> Value throws JsonParseError {
    try cursor.expect('{');
    cursor.skipWhitespace();

    var obj: [String: Value] = [:];

    guard let some (first, firstWidth) = cursor.peekChar() else {
        throw JsonParseError("unexpected end of input in object", cursor.pos)
    }

    // Empty object shortcut
    if first == '}' {
        cursor.pos = cursor.pos + firstWidth;
        return .Ok(Value.Obj(obj))
    }

    try parseMember(cursor, obj);

    loop {
        cursor.skipWhitespace();

        guard let some (c, width) = cursor.peekChar() else {
            throw JsonParseError("unexpected end of input in object", cursor.pos)
        }

        if c == '}' {
            cursor.pos = cursor.pos + width;
            return .Ok(Value.Obj(obj))
        }

        if c != ',' {
            throw JsonParseError("expected ',' or '}' in object", cursor.pos)
        }

        cursor.pos = cursor.pos + width;
        cursor.skipWhitespace();
        try parseMember(cursor, obj)
    }
}

/// Parses one `"key": value` pair at the cursor and inserts it into `obj`.
func parseMember(mutating cursor: JsonCursor, mutating obj: [String: Value]) -> () throws JsonParseError {
    let key = try parseRawString(cursor);
    cursor.skipWhitespace();
    try cursor.expect(':');
    cursor.skipWhitespace();
    let value = try parseValue(cursor);
    obj.insert(key, value);
    .Ok(())
}
