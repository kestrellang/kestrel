// URL and query string parsing

module http.url

/// A parsed URL split into path, query string, and path segments.
public struct ParsedUrl: Cloneable {
    public var path: String
    public var queryString: String
    public var segments: Array[String]

    public func clone() -> ParsedUrl {
        ParsedUrl(path: self.path.clone(), queryString: self.queryString.clone(), segments: self.segments.clone())
    }
}

/// Parses a raw request path like "/users/42?page=1&limit=10" into components.
public func parseUrl(raw: String) -> ParsedUrl {
    var path = raw;
    var queryString = String();

    // Split on "?" for query string
    match raw.find("?") {
        .Some(qIdx) => {
            path = raw.substringBytes(from: 0, to: qIdx);
            queryString = raw.substringBytes(from: qIdx + 1, to: raw.byteCount)
        },
        .None => {}
    }

    // Split path into segments (skip empty segments from leading/trailing slashes)
    let segments = splitSegments(path);

    ParsedUrl(path: path, queryString: queryString, segments: segments)
}

/// Parses a query string like "key=value&key2=value2" into an array of pairs.
public func parseQueryString(qs: String) -> Array[(String, String)] {
    var result = Array[(String, String)]();
    if qs.byteCount == 0 {
        return result
    }

    var pairs = qs.split("&");
    while let .Some(pair) = pairs.next() {
        match pair.find("=") {
            .Some(eqIdx) => {
                let key = percentDecode(pair.substringBytes(from: 0, to: eqIdx));
                let val = percentDecode(pair.substringBytes(from: eqIdx + 1, to: pair.byteCount));
                result.append((key, val))
            },
            .None => {
                // Key with no value
                result.append((percentDecode(pair), String()))
            }
        }
    }
    result
}

/// Percent-decodes a URL-encoded string (%20 -> space, etc.).
public func percentDecode(s: String) -> String {
    var result = String();
    let len = s.byteCount;
    var i: Int64 = 0;
    while i < len {
        let byte = s.bytes(unchecked: i);
        if byte == 37 and i + 2 < len { // '%'
            let hi = hexDigit(s.bytes(unchecked: i + 1));
            let lo = hexDigit(s.bytes(unchecked: i + 2));
            if hi >= 0 and lo >= 0 {
                let decoded = hi * 16 + lo;
                result.appendByte(UInt8(from: decoded));
                i = i + 3
            } else {
                result.appendByte(byte);
                i = i + 1
            }
        } else if byte == 43 { // '+' -> space
            result.append(" ");
            i = i + 1
        } else {
            result.appendByte(byte);
            i = i + 1
        }
    }
    result
}

/// Splits a path like "/users/42/posts" into ["users", "42", "posts"].
/// Skips empty segments.
func splitSegments(path: String) -> Array[String] {
    var segments = Array[String]();
    var parts = path.split("/");
    while let .Some(part) = parts.next() {
        if part.byteCount > 0 {
            segments.append(part)
        }
    }
    segments
}

/// Returns the numeric value of a hex digit (0-15), or -1 if invalid.
func hexDigit(byte: UInt8) -> Int64 {
    let b = Int64(from: byte);
    if b >= 48 and b <= 57 { return b - 48 }         // '0'-'9'
    if b >= 65 and b <= 70 { return b - 55 }         // 'A'-'F'
    if b >= 97 and b <= 102 { return b - 87 }        // 'a'-'f'
    return -1
}
