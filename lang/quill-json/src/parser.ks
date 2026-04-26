// JSON parser - recursive descent, byte-oriented

module quill.json.parser

import quill.value.(Value)
import quill.json.error.(JsonParseError)

// ============================================================================
// JSON CURSOR
// ============================================================================

/// Tracks position in the JSON source string during parsing.
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

    /// Returns the current byte, or None if at end.
    func peek() -> Optional[UInt8] {
        if self.pos < self.len {
            self.source.bytes(checked: self.pos)
        } else {
            .None
        }
    }

    /// Returns the current byte and advances. Errors if at end.
    mutating func advance() -> Result[UInt8, JsonParseError] {
        if self.pos < self.len {
            let b = self.source.bytes(unchecked: self.pos);
            self.pos = self.pos + 1;
            .Ok(b)
        } else {
            .Err(JsonParseError("unexpected end of input", self.pos))
        }
    }

    /// Advances without returning the byte.
    mutating func skip() {
        if self.pos < self.len {
            self.pos = self.pos + 1;
        }
    }

    /// Returns true if at end of input.
    func atEnd() -> Bool {
        self.pos >= self.len
    }

    /// Skips whitespace characters (space, tab, newline, carriage return).
    mutating func skipWhitespace() {
        while self.pos < self.len {
            let b = self.source.bytes(unchecked: self.pos);
            // space=32, tab=9, newline=10, carriage return=13
            if b == 32 or b == 9 or b == 10 or b == 13 {
                self.pos = self.pos + 1;
            } else {
                return
            }
        }
    }

    /// Expects and consumes a specific byte. Errors if mismatch.
    mutating func expect(byte: UInt8) -> Result[(), JsonParseError] {
        let b = try self.advance();
        if b == byte {
            .Ok(())
        } else {
            .Err(JsonParseError("expected '" + charForByte(byte) + "', got '" + charForByte(b) + "'", self.pos - 1))
        }
    }

    /// Expects and consumes a specific string. Errors if mismatch.
    mutating func expectStr(expected: String) -> Result[(), JsonParseError] {
        let startPos = self.pos;
        var i: Int64 = 0;
        while i < expected.byteCount {
            if self.pos >= self.len {
                return .Err(JsonParseError("unexpected end of input, expected '" + expected + "'", startPos))
            }
            let actual = self.source.bytes(unchecked: self.pos);
            let expectedByte = expected.bytes(unchecked: i);
            if actual != expectedByte {
                return .Err(JsonParseError("expected '" + expected + "'", startPos))
            }
            self.pos = self.pos + 1;
            i = i + 1
        }
        .Ok(())
    }
}

/// Helper to convert a byte to a displayable string.
func charForByte(b: UInt8) -> String {
    var s = String();
    s.appendByte(b);
    s
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
    match cursor.peek() {
        .Some(b) => {
            // 'n' = 110, 't' = 116, 'f' = 102, '"' = 34, '[' = 91, '{' = 123, '-' = 45
            if b == 110 {
                parseNull(cursor)
            } else if b == 116 {
                parseTrue(cursor)
            } else if b == 102 {
                parseFalse(cursor)
            } else if b == 34 {
                parseJsonString(cursor)
            } else if b == 91 {
                parseArray(cursor)
            } else if b == 123 {
                parseObject(cursor)
            } else if b == 45 or (b >= 48 and b <= 57) {
                // '-' or '0'-'9'
                parseNumber(cursor)
            } else {
                .Err(JsonParseError("unexpected character '" + charForByte(b) + "'", cursor.pos))
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
    match cursor.peek() {
        .Some(b) => {
            if b == 45 { // '-'
                cursor.skip()
            }
        },
        .None => {}
    }

    // Integer part
    match cursor.peek() {
        .Some(b) => {
            if b == 48 { // '0'
                cursor.skip()
            } else if b >= 49 and b <= 57 { // '1'-'9'
                cursor.skip();
                while cursor.pos < cursor.len {
                    let d = cursor.source.bytes(unchecked: cursor.pos);
                    if d >= 48 and d <= 57 {
                        cursor.skip()
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
    match cursor.peek() {
        .Some(b) => {
            if b == 46 { // '.'
                isFloat = true;
                cursor.skip();
                // Must have at least one digit after '.'
                var hasDigit = false;
                while cursor.pos < cursor.len {
                    let d = cursor.source.bytes(unchecked: cursor.pos);
                    if d >= 48 and d <= 57 {
                        cursor.skip();
                        hasDigit = true
                    } else {
                        break
                    }
                }
                if hasDigit == false {
                    return .Err(JsonParseError("expected digit after '.'", cursor.pos))
                }
            }
        },
        .None => {}
    }

    // Exponent part
    match cursor.peek() {
        .Some(b) => {
            if b == 101 or b == 69 { // 'e' or 'E'
                isFloat = true;
                cursor.skip();
                // Optional sign
                match cursor.peek() {
                    .Some(s) => {
                        if s == 43 or s == 45 { // '+' or '-'
                            cursor.skip()
                        }
                    },
                    .None => {}
                }
                // Must have at least one digit
                var hasDigit = false;
                while cursor.pos < cursor.len {
                    let d = cursor.source.bytes(unchecked: cursor.pos);
                    if d >= 48 and d <= 57 {
                        cursor.skip();
                        hasDigit = true
                    } else {
                        break
                    }
                }
                if hasDigit == false {
                    return .Err(JsonParseError("expected digit in exponent", cursor.pos))
                }
            }
        },
        .None => {}
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

/// Parses a JSON string (without wrapping in Value).
func parseRawString(mutating cursor: JsonCursor) -> Result[String, JsonParseError] {
    try cursor.expect(34); // '"'
    var result = String();

    while true {
        if cursor.atEnd() {
            return .Err(JsonParseError("unterminated string", cursor.pos))
        }
        let b = cursor.source.bytes(unchecked: cursor.pos);

        if b == 34 { // '"' - end of string
            cursor.skip();
            return .Ok(result)
        }

        if b == 92 { // '\' - escape sequence
            cursor.skip();
            let esc = try cursor.advance();
            // \" \\ \/ \b \f \n \r \t \uXXXX
            if esc == 34 { // \"
                result.appendByte(34)
            } else if esc == 92 { // \\
                result.appendByte(92)
            } else if esc == 47 { // \/
                result.appendByte(47)
            } else if esc == 98 { // \b
                result.appendByte(8)
            } else if esc == 102 { // \f
                result.appendByte(12)
            } else if esc == 110 { // \n
                result.appendByte(10)
            } else if esc == 114 { // \r
                result.appendByte(13)
            } else if esc == 116 { // \t
                result.appendByte(9)
            } else if esc == 117 { // \u
                let codepoint = try parseUnicodeEscape(cursor);
                result.appendChar(Char(UInt32(from: codepoint)))
            } else {
                return .Err(JsonParseError("invalid escape sequence", cursor.pos - 1))
            }
        } else {
            // Regular byte - copy it
            result.appendByte(b);
            cursor.skip()
        }
    }

    // Unreachable, but needed for type checker
    .Err(JsonParseError("unterminated string", cursor.pos))
}

/// Parses a 4-digit hex escape \uXXXX and returns the code point.
func parseUnicodeEscape(mutating cursor: JsonCursor) -> Result[Int64, JsonParseError] {
    var value: Int64 = 0;
    var i: Int64 = 0;
    while i < 4 {
        let b = try cursor.advance();
        let digit = hexDigitValue(b);
        if digit < 0 {
            return .Err(JsonParseError("invalid hex digit in unicode escape", cursor.pos - 1))
        }
        value = value * 16 + digit;
        i = i + 1
    }
    .Ok(value)
}

/// Returns the numeric value of a hex digit, or -1 if not a hex digit.
func hexDigitValue(b: UInt8) -> Int64 {
    let n = Int64(from: b);
    if n >= 48 and n <= 57 { // '0'-'9'
        n - 48
    } else if n >= 65 and n <= 70 { // 'A'-'F'
        n - 55
    } else if n >= 97 and n <= 102 { // 'a'-'f'
        n - 87
    } else {
        let neg: Int64 = 0 - 1;
        neg
    }
}

func parseArray(mutating cursor: JsonCursor) -> Result[Value, JsonParseError] {
    try cursor.expect(91); // '['
    cursor.skipWhitespace();

    var items = Array[Value]();

    // Check for empty array
    match cursor.peek() {
        .Some(b) => {
            if b == 93 { // ']'
                cursor.skip();
                return .Ok(Value.Arr(items))
            }
        },
        .None => return .Err(JsonParseError("unexpected end of input in array", cursor.pos))
    }

    // Parse first element
    let first = try parseValue(cursor);
    items.append(first);

    // Parse remaining elements
    while true {
        cursor.skipWhitespace();
        match cursor.peek() {
            .Some(b) => {
                if b == 93 { // ']'
                    cursor.skip();
                    return .Ok(Value.Arr(items))
                }
                if b == 44 { // ','
                    cursor.skip();
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
    try cursor.expect(123); // '{'
    cursor.skipWhitespace();

    var obj = Dictionary[String, Value]();

    // Check for empty object
    match cursor.peek() {
        .Some(b) => {
            if b == 125 { // '}'
                cursor.skip();
                return .Ok(Value.Obj(obj))
            }
        },
        .None => return .Err(JsonParseError("unexpected end of input in object", cursor.pos))
    }

    // Parse first key-value pair
    let firstKey = try parseRawString(cursor);
    cursor.skipWhitespace();
    try cursor.expect(58); // ':'
    cursor.skipWhitespace();
    let firstVal = try parseValue(cursor);
    let _ = obj.insert(firstKey, firstVal);

    // Parse remaining key-value pairs
    while true {
        cursor.skipWhitespace();
        match cursor.peek() {
            .Some(b) => {
                if b == 125 { // '}'
                    cursor.skip();
                    return .Ok(Value.Obj(obj))
                }
                if b == 44 { // ','
                    cursor.skip();
                    cursor.skipWhitespace();
                    let key = try parseRawString(cursor);
                    cursor.skipWhitespace();
                    try cursor.expect(58); // ':'
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

/// Parses a string as an Int64. Returns None on failure.
func parseInt64(s: String) -> Optional[Int64] {
    let len = s.byteCount;
    if len == 0 {
        return .None
    }

    var i: Int64 = 0;
    var negative = false;
    let firstByte = s.bytes(unchecked: 0);
    if firstByte == 45 { // '-'
        negative = true;
        i = 1
    }

    if i >= len {
        return .None
    }

    var result: Int64 = 0;
    while i < len {
        let b = Int64(from: s.bytes(unchecked: i));
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

/// Parses a string as a Float64. Returns None on failure.
/// Handles: integer part, fractional part, exponent.
func parseFloat64(s: String) -> Optional[Float64] {
    let len = s.byteCount;
    if len == 0 {
        return .None
    }

    var i: Int64 = 0;
    var negative = false;
    let firstByte = s.bytes(unchecked: 0);
    if firstByte == 45 { // '-'
        negative = true;
        i = 1
    }

    if i >= len {
        return .None
    }

    // Integer part
    var intPart: Float64 = 0.0;
    while i < len {
        let b = Int64(from: s.bytes(unchecked: i));
        if b >= 48 and b <= 57 {
            intPart = intPart * 10.0 + Float64(from: b - 48);
            i = i + 1
        } else {
            break
        }
    }

    // Fractional part
    var fracPart: Float64 = 0.0;
    var fracDiv: Float64 = 1.0;
    if i < len {
        let dotByte = s.bytes(unchecked: i);
        if dotByte == 46 { // '.'
            i = i + 1;
            while i < len {
                let b = Int64(from: s.bytes(unchecked: i));
                if b >= 48 and b <= 57 {
                    fracPart = fracPart * 10.0 + Float64(from: b - 48);
                    fracDiv = fracDiv * 10.0;
                    i = i + 1
                } else {
                    break
                }
            }
        }
    }

    var result = intPart + fracPart / fracDiv;

    // Exponent part
    if i < len {
        let eByte = s.bytes(unchecked: i);
        if eByte == 101 or eByte == 69 { // 'e' or 'E'
            i = i + 1;
            var expNeg = false;
            if i < len {
                let signByte = s.bytes(unchecked: i);
                if signByte == 43 { // '+'
                    i = i + 1
                } else if signByte == 45 { // '-'
                    expNeg = true;
                    i = i + 1
                }
            }
            var exp: Float64 = 0.0;
            while i < len {
                let b = Int64(from: s.bytes(unchecked: i));
                if b >= 48 and b <= 57 {
                    exp = exp * 10.0 + Float64(from: b - 48);
                    i = i + 1
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
