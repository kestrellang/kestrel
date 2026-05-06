/// URL parsing, query-string decoding, and percent-decoding.
///
/// Splits a raw HTTP request path into its structural components
/// (path, query string, segments) and provides helpers for decoding
/// percent-encoded and query-string data.
///
/// # Examples
///
/// ```
/// let u = parseUrl("/users/42?page=1&limit=10");
/// u.path;                // "/users/42"
/// u.queryString;         // "page=1&limit=10"
/// u.segments;            // ["users", "42"]
///
/// let qs = parseQueryString(u.queryString);
/// // [("page", "1"), ("limit", "10")]
/// ```

module http.url

import http.wire.(hexDigit, hexChar)

/// A parsed URL split into its path, raw query string, and path
/// segments.
///
/// Produced by `parseUrl`. The `path` retains leading/trailing
/// slashes; `segments` strips them so each element is a non-empty
/// path component.
///
/// # Examples
///
/// ```
/// let u = parseUrl("/api/v1/items?q=test");
/// u.path;           // "/api/v1/items"
/// u.queryString;    // "q=test"
/// u.segments;       // ["api", "v1", "items"]
/// ```
///
/// # Representation
///
/// Three fields: the path `String`, the raw query `String` (empty if
/// no `?`), and an `Array[String]` of non-empty path segments.
public struct ParsedUrl: Cloneable {
    /// The path portion of the URL, including any leading `/`.
    public var path: String
    /// The raw query string after `?`, or empty if none was present.
    public var queryString: String
    /// The non-empty segments of the path, split on `/`.
    public var segments: Array[String]

    public func clone() -> ParsedUrl {
        ParsedUrl(path: self.path.clone(), queryString: self.queryString.clone(), segments: self.segments.clone())
    }
}

/// Parses a raw HTTP request path into a `ParsedUrl`.
///
/// Splits on the first `?` to separate the path from the query
/// string, then splits the path on `/` to produce segments (empty
/// segments from leading/trailing slashes are dropped).
///
/// # Examples
///
/// ```
/// let u = parseUrl("/users/42?page=1");
/// u.path;           // "/users/42"
/// u.queryString;    // "page=1"
/// u.segments;       // ["users", "42"]
///
/// let root = parseUrl("/");
/// root.segments;    // []
/// ```
public func parseUrl(raw: String) -> ParsedUrl {
    var path = raw;
    var queryString = String();

    let rawSlice = raw.asSlice();
    match raw.firstIndex(of: "?") {
        .Some(qIdx) => {
            path = rawSlice.subslice(from: rawSlice.start, to: qIdx.value).toOwned();
            queryString = rawSlice.subslice(from: qIdx.value + 1, to: rawSlice.end).toOwned()
        },
        .None => {}
    }

    let segments = splitSegments(path);

    ParsedUrl(path: path, queryString: queryString, segments: segments)
}

/// Parses a query string into an array of `(key, value)` pairs.
///
/// Splits on `&`, then on the first `=` in each pair. Both keys and
/// values are percent-decoded. A pair without `=` produces a key with
/// an empty value.
///
/// # Examples
///
/// ```
/// parseQueryString("a=1&b=2");
/// // [("a", "1"), ("b", "2")]
///
/// parseQueryString("q=hello%20world");
/// // [("q", "hello world")]
///
/// parseQueryString("flag");
/// // [("flag", "")]
/// ```
public func parseQueryString(qs: String) -> Array[(String, String)] {
    var result = Array[(String, String)]();
    if qs.byteCount == 0 {
        return result
    }

    for pair in qs.split("&") {
        match pair.firstIndex(of: "=") {
            .Some(eqIdx) => {
                let key = percentDecode(pair.subslice(from: pair.start, to: eqIdx.value).toOwned());
                let val = percentDecode(pair.subslice(from: eqIdx.value + 1, to: pair.end).toOwned());
                result.append((key, val))
            },
            .None => {
                result.append((percentDecode(pair.toOwned()), String()))
            }
        }
    }
    result
}

/// Decodes a percent-encoded string.
///
/// Replaces `%XX` sequences with the corresponding byte and `+` with
/// a space (form-encoded convention). Invalid `%` sequences (e.g.
/// `%GG`) are passed through literally.
///
/// # Examples
///
/// ```
/// percentDecode("hello%20world");  // "hello world"
/// percentDecode("a+b");            // "a b"
/// percentDecode("100%25");         // "100%"
/// ```
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
        } else if byte == 43 { // '+' → space
            result.append(" ");
            i = i + 1
        } else {
            result.appendByte(byte);
            i = i + 1
        }
    }
    result
}

/// Percent-encodes a string for use in URLs and form data.
///
/// RFC 3986 unreserved characters (`A`–`Z`, `a`–`z`, `0`–`9`, `-`,
/// `_`, `.`, `~`) pass through unchanged. Spaces become `+` (the
/// form-encoding convention). Everything else is encoded as `%XX`.
/// The inverse of `percentDecode`.
///
/// # Examples
///
/// ```
/// percentEncode("hello world");  // "hello+world"
/// percentEncode("100%");         // "100%25"
/// percentEncode("a&b=c");        // "a%26b%3Dc"
/// ```
public func percentEncode(s: String) -> String {
    var result = String();
    for i in 0..<s.byteCount {
        let byte = s.bytes(unchecked: i);
        let b = Int64(from: byte);
        if (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or (b >= 48 and b <= 57)
            or b == 45 or b == 95 or b == 46 or b == 126 {
            result.appendByte(byte)
        } else if b == 32 {
            result.append("+")
        } else {
            result.append("%");
            result.appendByte(hexChar(b / 16));
            result.appendByte(hexChar(b % 16))
        }
    }
    result
}

/// Encodes an array of `(key, value)` pairs into a query string.
///
/// Both keys and values are percent-encoded. Pairs are joined with
/// `&`. The inverse of `parseQueryString`.
///
/// # Examples
///
/// ```
/// encodeQueryString([("q", "hello world"), ("page", "1")]);
/// // "q=hello+world&page=1"
///
/// encodeQueryString([("a&b", "c=d")]);
/// // "a%26b=c%3Dd"
/// ```
public func encodeQueryString(pairs: Array[(String, String)]) -> String {
    var result = String();
    var first = true;
    for (key, value) in pairs {
        if not first {
            result.append("&")
        }
        result.append(percentEncode(key));
        result.append("=");
        result.append(percentEncode(value));
        first = false
    }
    result
}

/// Splits a URL path on `/`, returning only non-empty segments.
///
/// # Examples
///
/// ```
/// splitSegments("/a/b/c");   // ["a", "b", "c"]
/// splitSegments("/");         // []
/// ```
func splitSegments(path: String) -> Array[String] {
    var segments = Array[String]();
    for part in path.split("/") {
        if part.byteCount > 0 {
            segments.append(part.toOwned())
        }
    }
    segments
}
