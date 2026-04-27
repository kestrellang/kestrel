// JSON parser - recursive descent over a Char cursor.

module quill.json.parser

import quill.value.(Value)
import quill.json.error.(JsonParseError)
import std.text.(decodeUtf8)

// ============================================================================
// JSON CURSOR
// ============================================================================

/// Tracks position in the JSON source string during parsing.
///
/// Indexes by *byte* offset (so substring slicing is O(1)) but exposes
/// `Char` for predicates — JSON's grammar is ASCII-only at structural
/// positions, but string contents and \uXXXX escapes are full Unicode.
struct JsonCursor: Cloneable {
    var source: String
    var pos: Int64
    var len: Int64

    init(source: String) {
        self.source = source;
        self.pos = 0;
        self.len = source.byteCount;
    }

    func clone() -> JsonCursor {
        JsonCursor(self.source.clone())
    }

    func atEnd() -> Bool {
        self.pos >= self.len
    }

    /// Returns the current code point and the number of bytes it consumes,
    /// or `.None` at end of input. Does **not** advance.
    //
    // Decodes UTF-8 directly from the source's byte buffer at `self.pos`.
    // Slicing with `substringBytes` here would copy O(len - pos) bytes per
    // call, making a parse run O(N²) on inputs of any size.
    func peekChar() -> Optional[(Char, Int64)] {
        if self.pos >= self.len {
            return .None
        }
        match decodeUtf8(self.source.bytes.asRaw(), self.len, at: self.pos) {
            .Some(decoded) => .Some((decoded.char, decoded.bytesConsumed)),
            .None => .None
        }
    }

    /// Returns the next code point and advances past it.
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

    /// Expects and consumes a specific code point.
    mutating func expect(c: Char) -> Result[(), JsonParseError] {
        let actual = try self.advanceChar();
        if actual == c {
            .Ok(())
        } else {
            var expected = String();
            expected.appendChar(c);
            var got = String();
            got.appendChar(actual);
            .Err(JsonParseError("expected '" + expected + "', got '" + got + "'", self.pos - 1))
        }
    }

    /// Expects and consumes a specific string literal.
    //
    // Compares bytes directly at the cursor offset; the previous implementation
    // sliced `source.substringBytes(from: pos, to: len)` first, copying the
    // entire tail of the input on every call.
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

/// Parses a JSON string into a Value.
public func parseJson(source: String) -> Result[Value, JsonParseError] {
    var cursor = JsonCursor(source);
    cursor.skipWhitespace();
    let value = try parseValue(cursor);
    cursor.skipWhitespace();
    if cursor.atEnd() == false {
        return .Err(JsonParseError("unexpected trailing content", cursor.pos))
    }
    .Ok(value)
}

// ============================================================================
// INTERNAL PARSERS
// ============================================================================

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
            } else if c == '-' or c.isDigit() {
                parseNumber(cursor)
            } else {
                var got = String();
                got.appendChar(c);
                .Err(JsonParseError("unexpected character '" + got + "'", cursor.pos))
            }
        },
        .None => .Err(JsonParseError("unexpected end of input", cursor.pos))
    }
}

func parseNull(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expectStr("null");
    .Ok(Value.Null)
}

func parseTrue(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expectStr("true");
    .Ok(Value.Boolean(true))
}

func parseFalse(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expectStr("false");
    .Ok(Value.Boolean(false))
}

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
            } else if c.isDigit() {
                cursor.pos = cursor.pos + pair.1;
                while let .Some(p) = cursor.peekChar() {
                    if p.0.isDigit() {
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
                if p.0.isDigit() {
                    cursor.pos = cursor.pos + p.1;
                    hasDigit = true
                } else {
                    break
                }
            }
            if hasDigit == false {
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
                if p.0.isDigit() {
                    cursor.pos = cursor.pos + p.1;
                    hasDigit = true
                } else {
                    break
                }
            }
            if hasDigit == false {
                return .Err(JsonParseError("expected digit in exponent", cursor.pos))
            }
        }
    }

    let numStr = cursor.source.substringBytes(from: start, to: cursor.pos);

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

func parseJsonString(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    let s = try parseRawString(cursor);
    .Ok(Value.Str(s))
}

/// Parses a JSON string literal (without wrapping in `Value`).
//
// Scans the source byte-by-byte for runs that are neither `"` (0x22) nor
// `\` (0x5C) and copies those runs into `result` as raw bytes. The previous
// implementation called `advanceChar` (decode UTF-8) + `appendChar` (re-encode
// UTF-8) for every character, which was the dominant per-char cost on
// string-heavy JSON. Bytes are already valid UTF-8 in the source, so direct
// passthrough preserves the encoding without re-encoding work. Escape
// sequences fall back to the slow path because `\uXXXX` may decode to a
// multi-byte code point.
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

        // Copy the run verbatim — no UTF-8 decode/encode overhead.
        var i = runStart;
        while i < cursor.pos {
            result.appendByte(srcBytes(unchecked: i));
            i = i + 1
        }

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
            result.appendByte(34)
        } else if esc == '\\' {
            result.appendByte(92)
        } else if esc == '/' {
            result.appendByte(47)
        } else if esc == 'b' {
            result.appendByte(8)
        } else if esc == 'f' {
            result.appendByte(12)
        } else if esc == 'n' {
            result.appendByte(10)
        } else if esc == 'r' {
            result.appendByte(13)
        } else if esc == 't' {
            result.appendByte(9)
        } else if esc == 'u' {
            let codepoint = try parseUnicodeEscape(cursor);
            result.appendChar(Char(UInt32(from: codepoint)))
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
    let _ = obj.insert(firstKey, firstVal);

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
                    let _ = obj.insert(key, val);
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
