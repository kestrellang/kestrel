// Path pattern matching and query string parsing
//
// This module provides route pattern compilation and matching,
// as well as URL query string parsing and decoding.

module expressks.internal.path;

import std.collections.dictionary;
import std.collections.array;
import std.text.string;

// Segment of a route pattern
public enum PathSegment {
    case Literal(value: String)     // Exact match like "users"
    case Param(name: String)        // Parameter like ":id"
    case Wildcard                   // Match anything "*"
}

// Compiled route pattern for efficient matching
public struct PathPattern {
    public var segments: Array[PathSegment];
    public var paramNames: Array[String];

    public init() {
        self.segments = Array[PathSegment]();
        self.paramNames = Array[String]();
    }

    // Compile a pattern string into a PathPattern
    // e.g., "/users/:id/posts/:postId" -> [Literal("users"), Param("id"), Literal("posts"), Param("postId")]
    public static func compile(pattern: String) -> PathPattern {
        var result = PathPattern();

        // Split on "/"
        for part in pattern.split(on: "/") {
            if part.isEmpty {
                continue;
            }

            if part.starts(with: ":") {
                // Parameter segment - extract name after ":"
                let paramName = substringFrom(str: part, start: 1);
                result.segments.append(.Param(name: paramName));
                result.paramNames.append(paramName);
            } else if part == "*" {
                result.segments.append(.Wildcard);
            } else {
                result.segments.append(.Literal(value: part));
            }
        }

        result
    }

    // Match a path against this pattern, extracting parameters
    // Returns None if no match, Some(params) if match
    public func match(path: String) -> Optional[Dictionary[String, String]] {
        var params = Dictionary[String, String]();

        // Collect path parts
        var pathParts = Array[String]();
        for part in path.split(on: "/") {
            if not part.isEmpty {
                pathParts.append(part);
            }
        }

        // Check if we have a wildcard at the end
        let hasWildcard = if self.segments.count > 0 {
            match self.segments(unchecked: self.segments.count - 1) {
                .Wildcard => true,
                _ => false
            }
        } else {
            false
        };

        // Check segment count
        if hasWildcard {
            // With wildcard, path must have at least as many parts as non-wildcard segments
            if pathParts.count < self.segments.count - 1 {
                return .None;
            }
        } else {
            // Without wildcard, counts must match exactly
            if pathParts.count != self.segments.count {
                return .None;
            }
        }

        // Match each segment
        for i in 0..<self.segments.count {
            match self.segments(unchecked: i) {
                .Literal(let expected) => {
                    if i >= pathParts.count {
                        return .None;
                    }
                    let actual = pathParts(unchecked: i);
                    if actual != expected {
                        return .None;
                    }
                },
                .Param(let name) => {
                    if i >= pathParts.count {
                        return .None;
                    }
                    let value = pathParts(unchecked: i);
                    params.insert(value: value, for: name);
                },
                .Wildcard => {
                    // Match rest of path - just stop here
                    break;
                }
            }
        }

        .Some(params)
    }
}

// Parse query string into dictionary
// e.g., "foo=bar&baz=qux" -> {"foo": "bar", "baz": "qux"}
public func parseQueryString(query: String) -> Dictionary[String, String] {
    var result = Dictionary[String, String]();

    if query.isEmpty {
        return result;
    }

    // Split on "&"
    for pair in query.split(on: "&") {
        if pair.isEmpty {
            continue;
        }

        // Find "=" position
        match pair.find(substring: "=") {
            .Some(let eqPos) => {
                let key = substringTo(str: pair, end: eqPos);
                let value = substringFrom(str: pair, start: eqPos + 1);
                result.insert(value: urlDecode(value), for: urlDecode(key));
            },
            .None => {
                // No value, just key
                result.insert(value: "", for: urlDecode(pair));
            }
        }
    }

    result
}

// URL decode (percent encoding)
// e.g., "hello%20world" -> "hello world"
public func urlDecode(input: String) -> String {
    var result = String();
    var i = 0;

    while i < input.byteCount {
        let ch = input.byteAt(index: i);

        if ch == 37 and i + 2 < input.byteCount {  // '%'
            let hi = hexDigitValue(input.byteAt(index: i + 1));
            let lo = hexDigitValue(input.byteAt(index: i + 2));

            match (hi, lo) {
                (.Some(let h), .Some(let l)) => {
                    let decoded = ((h << 4) | l) as UInt32;
                    result.append(codePoint: CodePoint(value: decoded));
                    i += 3;
                    continue;
                },
                _ => {
                    // Invalid escape, keep as-is
                }
            }
        } else if ch == 43 {  // '+' -> space
            result.append(codePoint: CodePoint(value: 32));
            i += 1;
            continue;
        }

        result.append(codePoint: CodePoint(value: ch as UInt32));
        i += 1;
    }

    result
}

// URL encode (percent encoding)
public func urlEncode(input: String) -> String {
    var result = String();

    for i in 0..<input.byteCount {
        let ch = input.byteAt(index: i);

        // Safe characters: alphanumeric, - _ . ~
        if (ch >= 65 and ch <= 90) or     // A-Z
           (ch >= 97 and ch <= 122) or    // a-z
           (ch >= 48 and ch <= 57) or     // 0-9
           ch == 45 or ch == 95 or ch == 46 or ch == 126 {  // - _ . ~
            result.append(codePoint: CodePoint(value: ch as UInt32));
        } else {
            // Encode as %XX
            result.append(codePoint: CodePoint(value: 37));  // '%'
            result.append(codePoint: CodePoint(value: hexDigitChar((ch >> 4) & 0x0F) as UInt32));
            result.append(codePoint: CodePoint(value: hexDigitChar(ch & 0x0F) as UInt32));
        }
    }

    result
}

// Helper: get hex digit value (0-15) from ASCII char
func hexDigitValue(ch: UInt8) -> Optional[UInt8] {
    if ch >= 48 and ch <= 57 {       // '0'-'9'
        .Some(ch - 48)
    } else if ch >= 65 and ch <= 70 { // 'A'-'F'
        .Some(ch - 55)
    } else if ch >= 97 and ch <= 102 { // 'a'-'f'
        .Some(ch - 87)
    } else {
        .None
    }
}

// Helper: get hex char from value (0-15)
func hexDigitChar(value: UInt8) -> UInt8 {
    if value < 10 {
        48 + value  // '0' + value
    } else {
        65 + value - 10  // 'A' + (value - 10)
    }
}

// Helper: extract substring from start to end of string
func substringFrom(str: String, start: Int) -> String {
    var result = String();
    for i in start..<str.byteCount {
        result.append(codePoint: CodePoint(value: str.byteAt(index: i) as UInt32));
    }
    result
}

// Helper: extract substring from beginning to end position
func substringTo(str: String, end: Int) -> String {
    var result = String();
    for i in 0..<end {
        result.append(codePoint: CodePoint(value: str.byteAt(index: i) as UInt32));
    }
    result
}

// Helper: extract substring between positions
func substring(str: String, from start: Int, to end: Int) -> String {
    var result = String();
    for i in start..<end {
        result.append(codePoint: CodePoint(value: str.byteAt(index: i) as UInt32));
    }
    result
}
