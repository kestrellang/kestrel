// JSON - JavaScript Object Notation
//
// This module provides JSON parsing and serialization using the serde framework.
//
// Usage:
// ```kestrel
// import std.json.(Json, JsonValue)
//
// // Parse JSON
// let value = /* try */Json.parse("{\"name\": \"Alice\", \"age\": 30}")
//
// // Access values
// let name = value["name"].asString()  // Optional["Alice"]
// let age = value["age"].asInt()       // Optional[30]
//
// // Serialize to JSON
// let json = Json.stringify(value)     // "{\"name\":\"Alice\",\"age\":30}"
//
// // Pretty print
// let pretty = Json.stringify(value, pretty: true)
//
// // Serialize any Serialize type
// let point = Point(x: 10, y: 20)
// let json = /* try */Json.serialize(point)  // "{\"x\":10,\"y\":20}"
//
// // Deserialize to any Deserialize type
// let point: Point = /* try */Json.deserialize("{\"x\":10,\"y\":20}")
// ```

module std.json

import std.serde.(
    Serialize,
    Deserialize,
    Serializer,
    Deserializer,
    ObjectSerializer,
    ArraySerializer,
    ObjectVisitor,
    ObjectAccess,
    ArrayAccess,
    SerializeError,
    DeserializeError
)
import std.result.(Result, Optional, Error)
import std.core.(Equatable, Hashable, UInt, UInt8, UInt16, UInt32, UInt64, Int8, Int16, Int32, Int64, Float32, Float64)
import std.collections.(Array, Dictionary)

// JsonError - errors during JSON operations
public struct JsonError: Error {
    public var message: String
    public var position: Optional[Int]

    public init(message: String) {
        self.message = message;
        self.position = .None
    }

    public init(message: String, at position: Int) {
        self.message = message;
        self.position = .Some(position)
    }

    public var description: String {
        match self.position {
            .Some(pos) => "JsonError at position " + pos.toString() + ": " + self.message,
            .None => "JsonError: " + self.message
        }
    }
}

// JsonValue - represents any JSON value
public enum JsonValue: Equatable {
    case Null
    case Bool(Bool)
    case Number(Float64)
    case String(String)
    case Array(Array[JsonValue])
    case Object(Dictionary[String, JsonValue])

    // Type checking
    public var isNull: Bool {
        match self {
            .Null => true,
            _ => false
        }
    }

    public var isBool: Bool {
        match self {
            .Bool(_) => true,
            _ => false
        }
    }

    public var isNumber: Bool {
        match self {
            .Number(_) => true,
            _ => false
        }
    }

    public var isString: Bool {
        match self {
            .String(_) => true,
            _ => false
        }
    }

    public var isArray: Bool {
        match self {
            .Array(_) => true,
            _ => false
        }
    }

    public var isObject: Bool {
        match self {
            .Object(_) => true,
            _ => false
        }
    }

    // Value extraction
    public func asBool() -> Optional[Bool] {
        match self {
            .Bool(v) => .Some(v),
            _ => .None
        }
    }

    public func asNumber() -> Optional[Float64] {
        match self {
            .Number(v) => .Some(v),
            _ => .None
        }
    }

    public func asInt() -> Optional[Int] {
        match self {
            .Number(v) => .Some(Int(v)),
            _ => .None
        }
    }

    public func asString() -> Optional[String] {
        match self {
            .String(v) => .Some(v),
            _ => .None
        }
    }

    public func asArray() -> Optional[Array[JsonValue]] {
        match self {
            .Array(v) => .Some(v),
            _ => .None
        }
    }

    public func asObject() -> Optional[Dictionary[String, JsonValue]] {
        match self {
            .Object(v) => .Some(v),
            _ => .None
        }
    }

    // Subscript access for objects
    //public subscript(key: String) -> JsonValue {
    //    match self {
    //        .Object(obj) => obj(key).unwrap(or: .Null),
    //        _ => .Null
    //    }
    //}

    // Subscript access for arrays
    //public subscript(index: Int) -> JsonValue {
    //    match self {
    //        .Array(arr) => arr(safe: index).unwrap(or: .Null),
    //        _ => .Null
    //    }
    //}

    // Equatable
    public func equals(other: JsonValue) -> Bool {
        match (self, other) {
            (.Null, .Null) => true,
            (.Bool(a), .Bool(b)) => a == b,
            (.Number(a), .Number(b)) => a == b,
            (.String(a), .String(b)) => a == b,
            (.Array(a), .Array(b)) => {
                if a.count != b.count { return false }
                /* for i in 0..<a.count {
                    if a(unchecked: i) != b(unchecked: i) { return false }
                } */
                true
            },
            (.Object(a), .Object(b)) => {
                if a.count != b.count { return false }
                /* for (key, value) in a {
                    match b(key) {
                        .Some(otherValue) => {
                            if value != otherValue { return false }
                        },
                        .None => return false
                    }
                } */
                true
            },
            _ => false
        }
    }
}

// JsonValue implements Serialize
extend JsonValue: Serialize {
    public mutating func serialize[S](mutating to serializer: S) -> Result[(), S.Error] where S: Serializer {
        match self {
            .Null => serializer.serializeNil(),
            .Bool(v) => serializer.serializeBool(value: v),
            .Number(v) => serializer.serializeFloat64(value: v),
            .String(v) => serializer.serializeString(value: v),
            .Array(arr) => {
                let arrSerializer = /* try */ serializer.beginArray(length: arr.count);
                /* for item in arr {
                    try arrSerializer.serializeElement(value: item)
                } */
                arrSerializer.end()
            },
            .Object(obj) => {
                let objSerializer = /*try*/ serializer.beginObject(name: "", fieldCount: obj.count);
                /* for (key, value) in obj {
                    try objSerializer.serializeField(name: key, value: value)
                } */
                objSerializer.end()
            }
        }
    }
}

// Json - main entry point for JSON operations
public struct Json {
    // Parse JSON string into JsonValue
    public static func parse(input: String) -> Result[JsonValue, JsonError] {
        var parser = JsonParser(input: input);
        parser.parse()
    }

    // Stringify JsonValue to JSON string
    public static func stringify(value: JsonValue) -> String {
        var writer = JsonWriter(pretty: false);
        writer.write(value: value);
        writer.output
    }

    // Stringify JsonValue with pretty printing
    public static func stringify(value: JsonValue, pretty: Bool) -> String {
        var writer = JsonWriter(pretty: pretty);
        writer.write(value: value);
        writer.output
    }

    // Serialize any Serialize type to JSON string
    public static func serialize[T](value: T) -> Result[String, JsonError] where T: Serialize {
        var serializer = JsonSerializer();
        match value.serialize(to: serializer) {
            .Ok(_) => serializer.finish(),
            .Err(e) => .Err(e)
        }
    }

    // Serialize with pretty printing
    public static func serialize[T](value: T, pretty: Bool) -> Result[String, JsonError] where T: Serialize {
        var serializer = JsonSerializer(pretty: pretty);
        match value.serialize(to: serializer) {
            .Ok(_) => serializer.finish(),
            .Err(e) => .Err(e)
        }
    }

    // Deserialize JSON string to any Deserialize type
    public static func deserialize[T](input: String) -> Result[T, JsonError] where T: Deserialize {
        var deserializer = JsonDeserializer(input: input);
        T.deserialize(from: deserializer)
    }
}

// JsonParser - parses JSON strings into JsonValue
struct JsonParser {
    private var input: String
    private var position: Int

    public init(input: String) {
        self.input = input;
        self.position = 0
    }

    public func parse() -> Result[JsonValue, JsonError] {
        self.skipWhitespace();
        let value = /* try */self.parseValue();
        self.skipWhitespace();
        if self.position < self.input.byteCount {
            return .Err(JsonError(message: "unexpected characters after JSON value", at: self.position))
        }
        .Ok(value)
    }

    private func parseValue() -> Result[JsonValue, JsonError] {
        self.skipWhitespace();

        if self.isAtEnd() {
            return .Err(JsonError(message: "unexpected end of input", at: self.position))
        }

        let ch = self.peek();

        if ch == 110 { // 'n'
            return self.parseNull()
        }
        if ch == 116 { // 't'
            return self.parseTrue()
        }
        if ch == 102 { // 'f'
            return self.parseFalse()
        }
        if ch == 34 { // '"'
            return self.parseString().map { JsonValue.String(it) }
        }
        if ch == 91 { // '['
            return self.parseArray()
        }
        if ch == 123 { // '{'
            return self.parseObject()
        }
        if ch == 45 or (ch >= 48 and ch <= 57) { // '-' or digit
            return self.parseNumber()
        }

        .Err(JsonError(message: "unexpected character: " + String(codePoints: [CodePoint(value: UInt32(ch))]), at: self.position))
    }

    private func parseNull() -> Result[JsonValue, JsonError] {
        if self.consumeKeyword("null") {
            .Ok(.Null)
        } else {
            .Err(JsonError(message: "expected 'null'", at: self.position))
        }
    }

    private func parseTrue() -> Result[JsonValue, JsonError] {
        if self.consumeKeyword("true") {
            .Ok(.Bool(true))
        } else {
            .Err(JsonError(message: "expected 'true'", at: self.position))
        }
    }

    private func parseFalse() -> Result[JsonValue, JsonError] {
        if self.consumeKeyword("false") {
            .Ok(.Bool(false))
        } else {
            .Err(JsonError(message: "expected 'false'", at: self.position))
        }
    }

    private func parseString() -> Result[String, JsonError] {
        if not self.consume(34) { // '"'
            return .Err(JsonError(message: "expected '\"'", at: self.position))
        }

        var result = String();

        while not self.isAtEnd() {
            let ch = self.peek();

            if ch == 34 { // '"'
                self.advance();
                return .Ok(result)
            }

            if ch == 92 { // '\'
                self.advance();
                if self.isAtEnd() {
                    return .Err(JsonError(message: "unexpected end of string", at: self.position))
                }
                let escaped = self.peek();
                self.advance();

                match escaped {
                    34 => result.append(codePoint: CodePoint(value: 34)),   // \"
                    92 => result.append(codePoint: CodePoint(value: 92)),   // \\
                    47 => result.append(codePoint: CodePoint(value: 47)),   // \/
                    98 => result.append(codePoint: CodePoint(value: 8)),    // \b
                    102 => result.append(codePoint: CodePoint(value: 12)),  // \f
                    110 => result.append(codePoint: CodePoint(value: 10)),  // \n
                    114 => result.append(codePoint: CodePoint(value: 13)),  // \r
                    116 => result.append(codePoint: CodePoint(value: 9)),   // \t
                    117 => { // \uXXXX
                        let hex = /* try */self.parseHexEscape();
                        result.append(codePoint: CodePoint(value: hex))
                    },
                    _ => return .Err(JsonError(message: "invalid escape sequence", at: self.position - 1))
                }
            } else if ch < 32 {
                return .Err(JsonError(message: "control character in string", at: self.position))
            } else {
                // Regular character - decode UTF-8
                if let (cp, len) = decodeUtf8(bytes: self.input.bytes.asSlice(), at: self.position) {
                    result.append(codePoint: cp);
                    self.position = self.position + len
                } else {
                    return .Err(JsonError(message: "invalid UTF-8", at: self.position))
                }
            }
        }

        .Err(JsonError(message: "unterminated string", at: self.position))
    }

    private func parseHexEscape() -> Result[UInt32, JsonError] {
        var value: UInt32 = 0;
        /* for _ in 0..<4 {
            if self.isAtEnd() {
                return .Err(JsonError(message: "incomplete unicode escape", at: self.position))
            }
            let ch = self.peek()
            self.advance()

            let digit: UInt32 = if ch >= 48 and ch <= 57 { // 0-9
                UInt32(ch - 48)
            } else if ch >= 65 and ch <= 70 { // A-F
                UInt32(ch - 55)
            } else if ch >= 97 and ch <= 102 { // a-f
                UInt32(ch - 87)
            } else {
                return .Err(JsonError(message: "invalid hex digit", at: self.position - 1))
            }

            value = (value << 4) | digit
        } */
        .Ok(value)
    }

    private func parseNumber() -> Result[JsonValue, JsonError] {
        let start = self.position;
        var isFloat = false;

        // Optional negative sign
        if self.peek() == 45 { // '-'
            self.advance()
        }

        // Integer part
        if self.isAtEnd() {
            return .Err(JsonError(message: "unexpected end of number", at: self.position));
        }

        if self.peek() == 48 { // '0'
            self.advance()
        } else if self.peek() >= 49 and self.peek() <= 57 { // 1-9
            while not self.isAtEnd() and self.peek() >= 48 and self.peek() <= 57 {
                self.advance()
            }
        } else {
            return .Err(JsonError(message: "invalid number", at: self.position))
        }

        // Fractional part
        if not self.isAtEnd() and self.peek() == 46 { // '.'
            isFloat = true;
            self.advance();
            if self.isAtEnd() or self.peek() < 48 or self.peek() > 57 {
                return .Err(JsonError(message: "expected digit after decimal point", at: self.position))
            }
            while not self.isAtEnd() and self.peek() >= 48 and self.peek() <= 57 {
                self.advance()
            }
        }

        // Exponent part
        if not self.isAtEnd() and (self.peek() == 101 or self.peek() == 69) { // 'e' or 'E'
            isFloat = true;
            self.advance();
            if not self.isAtEnd() and (self.peek() == 43 or self.peek() == 45) { // '+' or '-'
                self.advance()
            }
            if self.isAtEnd() or self.peek() < 48 or self.peek() > 57 {
                return .Err(JsonError(message: "expected digit in exponent", at: self.position))
            }
            while not self.isAtEnd() and self.peek() >= 48 and self.peek() <= 57 {
                self.advance()
            }
        }

        // Extract the number string and parse
        let numStr = self.input.substringBytes(from: start, to: self.position);
        match Float64.parse(numStr) {
            .Some(num) => .Ok(.Number(num)),
            .None => .Err(JsonError(message: "invalid number", at: start))
        }
    }

    private func parseArray() -> Result[JsonValue, JsonError] {
        if not self.consume(91) { // '['
            return .Err(JsonError(message: "expected '['", at: self.position))
        }

        var elements: Array[JsonValue] = [];

        self.skipWhitespace();

        if not self.isAtEnd() and self.peek() == 93 { // ']'
            self.advance();
            return .Ok(.Array(elements))
        }

        loop {
            let value = /* try */self.parseValue();
            elements.append(value);

            self.skipWhitespace();

            if self.isAtEnd() {
                return .Err(JsonError(message: "unterminated array", at: self.position));
            }

            if self.peek() == 93 { // ']'
                self.advance();
                return .Ok(.Array(elements));
            }

            if not self.consume(44) { // ','
                return .Err(JsonError(message: "expected ',' or ']'", at: self.position))
            }

            self.skipWhitespace()
        }
    }

    private func parseObject() -> Result[JsonValue, JsonError] {
        if not self.consume(123) { // '{'
            return .Err(JsonError(message: "expected '{'", at: self.position))
        }

        var entries: Dictionary[String, JsonValue] = [];

        self.skipWhitespace();

        if not self.isAtEnd() and self.peek() == 125 { // '}'
            self.advance();
            return .Ok(.Object(entries))
        }

        loop {
            self.skipWhitespace();

            if self.isAtEnd() or self.peek() != 34 { // '"'
                return .Err(JsonError(message: "expected string key", at: self.position))
            }

            let key = /* try */self.parseString();

            self.skipWhitespace();

            if not self.consume(58) { // ':'
                return .Err(JsonError(message: "expected ':'", at: self.position));
            }

            let value = /* try */self.parseValue();
            entries(key) = value;

            self.skipWhitespace();

            if self.isAtEnd() {
                return .Err(JsonError(message: "unterminated object", at: self.position));
            }

            if self.peek() == 125 { // '}'
                self.advance();
                return .Ok(.Object(entries))
            }

            if not self.consume(44) { // ','
                return .Err(JsonError(message: "expected ',' or '}'", at: self.position));
            }
        }
    }

    // Helper methods
    private func isAtEnd() -> Bool {
        self.position >= self.input.byteCount
    }

    private func peek() -> UInt8 {
        self.input.byteAt(index: self.position)
    }

    private func advance() {
        self.position = self.position + 1
    }

    private func consume(expected: UInt8) -> Bool {
        if self.isAtEnd() or self.peek() != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    private func consumeKeyword(keyword: String) -> Bool {
        if self.position + keyword.byteCount > self.input.byteCount {
            return false
        }
        /* for i in 0..<keyword.byteCount {
            if self.input.byteAt(index: self.position + i) != keyword.byteAt(index: i) {
                return false
            }
        } */
        self.position = self.position + keyword.byteCount;
        true
    }

    private func skipWhitespace() {
        while not self.isAtEnd() {
            let ch = self.peek();
            if ch == 32 or ch == 9 or ch == 10 or ch == 13 { // space, tab, newline, carriage return
                self.advance()
            } else {
                break
            }
        }
    }
}

// JsonWriter - writes JsonValue to JSON string
struct JsonWriter {
    public var output: String
    private var pretty: Bool
    private var indent: Int

    public init(pretty: Bool) {
        self.output = String();
        self.pretty = pretty;
        self.indent = 0
    }

    public func write(value: JsonValue) {
        match value {
            .Null => self.output.append(string: "null"),
            .Bool(v) => self.output.append(string: if v { "true" } else { "false" }),
            .Number(v) => self.output.append(string: v.toString()),
            .String(v) => self.writeString(v),
            .Array(arr) => self.writeArray(arr),
            .Object(obj) => self.writeObject(obj)
        }
    }

    private func writeString(value: String) {
        self.output.append(codePoint: CodePoint(value: 34)); // '"'

        /* for cp in value.codePoints {
            let v = cp.value;
            if v == 34 { // '"'
                self.output.append(string: "\\\"")
            } else if v == 92 { // '\'
                self.output.append(string: "\\\\")
            } else if v == 8 { // backspace
                self.output.append(string: "\\b")
            } else if v == 12 { // form feed
                self.output.append(string: "\\f")
            } else if v == 10 { // newline
                self.output.append(string: "\\n")
            } else if v == 13 { // carriage return
                self.output.append(string: "\\r")
            } else if v == 9 { // tab
                self.output.append(string: "\\t")
            } else if v < 32 { // control characters
                self.output.append(string: "\\u")
                self.writeHex4(v)
            } else {
                self.output.append(codePoint: cp)
            }
        } */

        self.output.append(codePoint: CodePoint(value: 34)) // '"'
    }

    private func writeHex4(value: UInt32) {
        let hexChars = "0123456789abcdef";
        /* for i in [12, 8, 4, 0] {
            let digit = Int((value >> i) & 0xF)
            self.output.append(codePoint: hexChars.codePoints.nth(digit).unwrap())
        } */
    }

    private func writeArray(arr: Array[JsonValue]) {
        self.output.append(codePoint: CodePoint(value: 91)); // '['

        if arr.isEmpty {
            self.output.append(codePoint: CodePoint(value: 93)); // ']'
            return
        }

        if self.pretty {
            self.indent = self.indent + 1;
            self.writeNewline()
        }

        /* for i in 0..<arr.count {
            if i > 0 {
                self.output.append(codePoint: CodePoint(value: 44)); // ','
                if self.pretty {
                    self.writeNewline()
                }
            }
            self.write(value: arr(unchecked: i))
        } */

        if self.pretty {
            self.indent = self.indent - 1;
            self.writeNewline()
        }

        self.output.append(codePoint: CodePoint(value: 93)) // ']'
    }

    private func writeObject(obj: Dictionary[String, JsonValue]) {
        self.output.append(codePoint: CodePoint(value: 123)); // '{'

        if obj.isEmpty {
            self.output.append(codePoint: CodePoint(value: 125)); // '}'
            return;
        }

        if self.pretty {
            self.indent = self.indent + 1;
            self.writeNewline();
        }

        var first = true;
        /* for (key, value) in obj {
            if not first {
                self.output.append(codePoint: CodePoint(value: 44)) // ','
                if self.pretty {
                    self.writeNewline()
                }
            }
            first = false

            self.writeString(key)
            self.output.append(codePoint: CodePoint(value: 58)) // ':'
            if self.pretty {
                self.output.append(codePoint: CodePoint(value: 32)) // ' '
            }
            self.write(value: value)
        } */

        if self.pretty {
            self.indent = self.indent - 1;
            self.writeNewline()
        }

        self.output.append(codePoint: CodePoint(value: 125)); // '}'
    }

    private func writeNewline() {
        self.output.append(codePoint: CodePoint(value: 10)); // '\n'
        /* for _ in 0..<(self.indent * 2) {
            self.output.append(codePoint: CodePoint(value: 32)); // ' '
        } */
    }
}

// JsonSerializer - implements Serializer for JSON
public struct JsonSerializer: Serializer {
    type Output = String
    type Error = JsonError

    private var writer: JsonWriter
    private var stack: Array[JsonContext]

    enum JsonContext {
        case Root
        case Array(count: Int)
        case Object(count: Int)
    }

    public init() {
        self.writer = JsonWriter(pretty: false);
        self.stack = [.Root];
    }

    public init(pretty: Bool) {
        self.writer = JsonWriter(pretty: pretty);
        self.stack = [.Root];
    }

    // Primitive serialization
    public mutating func serializeNil() -> Result[(), JsonError] {
        self.writer.write(value: .Null);
        .Ok(());
    }

    public mutating func serializeBool(value: Bool) -> Result[(), JsonError] {
        self.writer.write(value: .Bool(value));
        .Ok(());
    }

    public mutating func serializeInt(value: Int) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(());
    }

    public mutating func serializeInt8(value: Int8) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(());
    }

    public mutating func serializeInt16(value: Int16) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeInt32(value: Int32) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeInt64(value: Int64) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeUInt(value: UInt) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeUInt8(value: UInt8) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeUInt16(value: UInt16) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeUInt32(value: UInt32) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeUInt64(value: UInt64) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeFloat32(value: Float32) -> Result[(), JsonError] {
        self.writer.write(value: .Number(Float64(value)));
        .Ok(())
    }

    public mutating func serializeFloat64(value: Float64) -> Result[(), JsonError] {
        self.writer.write(value: .Number(value));
        .Ok(())
    }

    public mutating func serializeString(value: String) -> Result[(), JsonError] {
        self.writer.write(value: .String(value));
        .Ok(())
    }

    // Compound types
    public mutating func serializeArray[S](values: Array[S]) -> Result[(), JsonError] where S: Serialize {
        var arr = /* try */self.beginArray(length: values.count);
        /* for item in values {
            try arr.serializeElement(value: item)
        } */
        arr.end()
    }

    public mutating func serializeMap[K, V](entries: Array[(K, V)]) -> Result[(), JsonError] where K: Serialize, V: Serialize {
        var obj = /* try */self.beginObject(name: "", fieldCount: entries.count);
        /* for (key, value) in entries {
            // For JSON, keys should be strings
            // This is a simplification; full impl would serialize key to string
            try obj.serializeField(name: key.toString(), value: value)
        } */
        obj.end()
    }

    public mutating func beginObject(name: String, fieldCount: Int) -> Result[JsonObjectSerializer, JsonError] {
        .Ok(JsonObjectSerializer(serializer: self, fieldCount: fieldCount))
    }

    public mutating func beginArray(length: Int) -> Result[JsonArraySerializer, JsonError] {
        .Ok(JsonArraySerializer(serializer: self, length: length))
    }

    public mutating func finish() -> Result[String, JsonError] {
        .Ok(self.writer.output)
    }
}

// JsonObjectSerializer - serializes object fields
public struct JsonObjectSerializer: ObjectSerializer {
    type Error = JsonError

    private var serializer: /*ref*/ JsonSerializer
    private var fieldCount: Int
    private var currentField: Int

    public init(serializer: JsonSerializer, fieldCount: Int) {
        self.serializer = serializer;
        self.fieldCount = fieldCount;
        self.currentField = 0;
        self.serializer.writer.output.append(codePoint: CodePoint(value: 123)) // '{'
    }

    public mutating func serializeField[V](name: String, value: V) -> Result[(), JsonError] where V: Serialize {
        if self.currentField > 0 {
            self.serializer.writer.output.append(codePoint: CodePoint(value: 44)); // ','
        }
        self.serializer.writer.write(value: .String(name));
        self.serializer.writer.output.append(codePoint: CodePoint(value: 58)); // ':'
        value.serialize(to: self.serializer);
        self.currentField = self.currentField + 1;
        .Ok(())
    }

    public mutating func end() -> Result[(), JsonError] {
        self.serializer.writer.output.append(codePoint: CodePoint(value: 125)); // '}'
        .Ok(())
    }
}

// JsonArraySerializer - serializes array elements
public struct JsonArraySerializer: ArraySerializer {
    type Error = JsonError

    private var serializer: /*ref*/ JsonSerializer
    private var length: Int
    private var currentIndex: Int

    public init(serializer: JsonSerializer, length: Int) {
        self.serializer = serializer;
        self.length = length;
        self.currentIndex = 0;
        self.serializer.writer.output.append(codePoint: CodePoint(value: 91)) // '['
    }

    public mutating func serializeElement[V](value: V) -> Result[(), JsonError] where V: Serialize {
        if self.currentIndex > 0 {
            self.serializer.writer.output.append(codePoint: CodePoint(value: 44)); // ','
        }
        value.serialize(to: self.serializer);
        self.currentIndex = self.currentIndex + 1;
        .Ok(())
    }

    public mutating func end() -> Result[(), JsonError] {
        self.serializer.writer.output.append(codePoint: CodePoint(value: 93)); // ']'
        .Ok(())
    }
}

// JsonDeserializer - implements Deserializer for JSON
public struct JsonDeserializer: Deserializer {
    type Error = JsonError

    private var parser: JsonParser
    private var peeked: Optional[JsonValue]

    public init(input: String) {
        self.parser = JsonParser(input: input);
        self.peeked = .None
    }

    private func peekValue() -> Result[JsonValue, JsonError] {
        if let value = self.peeked {
            return .Ok(value)
        }
        let value = /* try */self.parser.parseValue();
        self.peeked = .Some(value);
        .Ok(value)
    }

    private func takeValue() -> Result[JsonValue, JsonError] {
        if let value = self.peeked {
            self.peeked = .None;
            return .Ok(value)
        }
        self.parser.parseValue()
    }

    // Primitive deserialization
    public mutating func deserializeNil() -> Result[(), JsonError] {
        let value = /* try */match self.takeValue() {
            .Ok(value) => value,
            .Err(e) => return .Err(e)
        };
        match value {
            .Null => .Ok(()),
            _ => .Err(JsonError(message: "expected null"))
        }
    }

    public mutating func deserializeBool() -> Result[Bool, JsonError] {
        let value = /* try */match self.takeValue()  {
            .Ok(value) => value,
            .Err(e) => return .Err(e)
        };
        match value {
            .Bool(v) => .Ok(v),
            _ => .Err(JsonError(message: "expected boolean"))
        }
    }

    public mutating func deserializeInt() -> Result[Int, JsonError] {
        let value = /* try */match self.takeValue()  {
            .Ok(value) => value,
            .Err(e) => return .Err(e)
        };
        match value {
            .Number(v) => .Ok(Int(v)),
            _ => .Err(JsonError(message: "expected integer"))
        }
    }

    public mutating func deserializeInt8() -> Result[Int8, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(Int8(v)),
            _ => .Err(JsonError(message: "expected integer"))
        }
    }

    public mutating func deserializeInt16() -> Result[Int16, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(Int16(v)),
            _ => .Err(JsonError(message: "expected integer"))
        }
    }

    public mutating func deserializeInt32() -> Result[Int32, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(Int32(v)),
            _ => .Err(JsonError(message: "expected integer"))
        }
    }

    public mutating func deserializeInt64() -> Result[Int64, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(Int64(v)),
            _ => .Err(JsonError(message: "expected integer"))
        }
    }

    public mutating func deserializeUInt() -> Result[UInt, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(UInt(v)),
            _ => .Err(JsonError(message: "expected unsigned integer"))
        }
    }

    public mutating func deserializeUInt8() -> Result[UInt8, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(UInt8(v)),
            _ => .Err(JsonError(message: "expected unsigned integer"))
        }
    }

    public mutating func deserializeUInt16() -> Result[UInt16, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(UInt16(v)),
            _ => .Err(JsonError(message: "expected unsigned integer"))
        }
    }

    public mutating func deserializeUInt32() -> Result[UInt32, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(UInt32(v)),
            _ => .Err(JsonError(message: "expected unsigned integer"))
        }
    }

    public mutating func deserializeUInt64() -> Result[UInt64, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(UInt64(v)),
            _ => .Err(JsonError(message: "expected unsigned integer"))
        }
    }

    public mutating func deserializeFloat32() -> Result[Float32, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(Float32(v)),
            _ => .Err(JsonError(message: "expected float"))
        }
    }

    public mutating func deserializeFloat64() -> Result[Float64, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .Number(v) => .Ok(v),
            _ => .Err(JsonError(message: "expected float"))
        }
    }

    public mutating func deserializeString() -> Result[String, JsonError] {
        let value = /* try */self.takeValue();
        match value {
            .String(v) => .Ok(v),
            _ => .Err(JsonError(message: "expected string"))
        }
    }

    // Compound types
    public mutating func deserializeArray[T]() -> Result[Array[T], JsonError] where T: Deserialize {
        let value = /* try */self.takeValue();
        match value {
            .Array(arr) => {
                var result: Array[T] = [];
                /* for item in arr {
                    // Re-serialize and deserialize each item
                    // This is inefficient but type-safe
                    var itemDeserializer = JsonDeserializer(input: Json.stringify(item));
                    let deserialized = /* try */T.deserialize(from: itemDeserializer);
                    result.append(deserialized);
                } */
                .Ok(result)
            },
            _ => .Err(JsonError(message: "expected array"))
        }
    }

    public mutating func deserializeMap[K, V]() -> Result[Dictionary[K, V], JsonError] where K: Deserialize, K: Hashable, V: Deserialize {
        let value = /* try */self.takeValue();
        match value {
            .Object(obj) => {
                var result: Dictionary[K, V] = [];
                /* for (key, val) in obj {
                    // Deserialize key and value
                    var keyDeserializer = JsonDeserializer(input: "\"" + key + "\"");
                    let deserializedKey = /* try */K.deserialize(from: keyDeserializer);

                    var valDeserializer = JsonDeserializer(input: Json.stringify(val));
                    let deserializedVal = /* try */V.deserialize(from: valDeserializer);

                    result(deserializedKey) = deserializedVal
                } */
                .Ok(result)
            },
            _ => .Err(JsonError(message: "expected object"))
        }
    }

    public mutating func deserializeObject[V](visitor: V.Visitor) -> Result[V, JsonError] where V: Deserialize {
        let value = /* try */self.takeValue();
        match value {
            .Object(obj) => {
                var access = JsonObjectAccess(object: obj);
                visitor.visit(access: access);
            },
            _ => .Err(JsonError(message: "expected object"))
        }
    }
}

// JsonObjectAccess - provides access to JSON object fields during deserialization
public struct JsonObjectAccess: ObjectAccess {
    type Error = JsonError

    private var object: Dictionary[String, JsonValue]
    private var keys: Array[String]
    private var index: Int
    private var currentKey: Optional[String]

    public init(object: Dictionary[String, JsonValue]) {
        self.object = object;
        self.keys = object.keys.collect[Array[String]]();
        self.index = 0;
        self.currentKey = .None
    }

    public mutating func nextField() -> Result[Optional[String], JsonError] {
        if self.index >= self.keys.count {
            return .Ok(.None)
        }
        let key = self.keys(unchecked: self.index);
        self.currentKey = .Some(key);
        self.index = self.index + 1;
        .Ok(.Some(key))
    }

    public mutating func value[V]() -> Result[V, JsonError] where V: Deserialize {
        match self.currentKey {
            .Some(key) => {
                match self.object(key) {
                    .Some(val) => {
                        var deserializer = JsonDeserializer(input: Json.stringify(val));
                        V.deserialize(from: deserializer);
                    },
                    .None => .Err(JsonError(message: "no value for key: " + key))
                }
            },
            .None => .Err(JsonError(message: "no current field"))
        }
    }

    public mutating func skipValue() -> Result[(), JsonError] {
        // Just move on - value will be ignored
        .Ok(())
    }
}

// JsonValue implements Deserialize
extend JsonValue: Deserialize {
    type Visitor = JsonValueVisitor

    public static func deserialize[D](mutating from deserializer: D) -> Result[JsonValue, D.Error] where D: Deserializer {
        // Special case: JsonDeserializer can directly return JsonValue
        // For other deserializers, we'd need to construct from primitives
        deserializer.deserializeAny()
    }
}

public struct JsonValueVisitor: ObjectVisitor {
    type Value = JsonValue

    public mutating func visit[A](mutating access: A) -> Result[JsonValue, A.Error] where A: ObjectAccess {
        var entries: Dictionary[String, JsonValue] = [];
        while let field = /* try */access.nextField() {
            let value = /* try */access.value[JsonValue]();
            entries(field) = value
        }
        .Ok(.Object(entries))
    }
}

// TODO: Protocol extensions not yet supported
// Add deserializeAny method to Deserializer for JsonValue support
// extend Deserializer {
//     public mutating func deserializeAny() -> Result[JsonValue, Error] {
//         // Default implementation - specific deserializers can override
//         .Err(Error(message: "deserializeAny not supported"))
//     }
// }

extend JsonDeserializer {
    public mutating func deserializeAny() -> Result[JsonValue, JsonError] {
        self.takeValue()
    }
}
