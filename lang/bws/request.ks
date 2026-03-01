// HTTP request parsing
//
// Provides HttpMethod, HttpRequest, and a request parser.

module bws.request

import std.num.(Int64, Int32, UInt8)
import std.result.(Result, Optional)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.net.libc.(recv)
import std.io.error.(Error, invalidInput)

// ============================================================================
// HTTP METHOD
// ============================================================================

/// HTTP request methods.
public enum HttpMethod {
    case Get
    case Post
    case Put
    case Delete
    case Patch
    case Head
    case Options

    /// Returns the method name as a string.
    public func toString() -> String {
        match self {
            .Get => "GET",
            .Post => "POST",
            .Put => "PUT",
            .Delete => "DELETE",
            .Patch => "PATCH",
            .Head => "HEAD",
            .Options => "OPTIONS"
        }
    }
}

/// Parses a string into an HttpMethod, or None if unrecognized.
public func parseMethod(s: String) -> HttpMethod? {
    if s.equals("GET") {
        return .Some(.Get)
    }
    if s.equals("POST") {
        return .Some(.Post)
    }
    if s.equals("PUT") {
        return .Some(.Put)
    }
    if s.equals("DELETE") {
        return .Some(.Delete)
    }
    if s.equals("PATCH") {
        return .Some(.Patch)
    }
    if s.equals("HEAD") {
        return .Some(.Head)
    }
    if s.equals("OPTIONS") {
        return .Some(.Options)
    }
    .None
}

// ============================================================================
// HTTP REQUEST
// ============================================================================

/// A parsed HTTP request.
public struct HttpRequest {
    public var method: HttpMethod
    public var path: String
    public var headers: Array[(String, String)]
    public var body: String
    public var queryString: String

    /// Returns the value of a header by name (case-sensitive), or None if not present.
    public func header(name: String) -> String? {
        var i: Int64 = 0;
        while i < self.headers.count {
            let pair = self.headers(unchecked: i);
            if pair.0.equals(name) {
                return .Some(pair.1)
            }
            i = i + 1
        }
        .None
    }
}

// ============================================================================
// REQUEST PARSING
// ============================================================================

/// Reads and parses an HTTP request from a raw socket fd.
///
/// Reads bytes until the header terminator (\r\n\r\n) is found,
/// then parses the request line, headers, and optional body.
/// Maximum header size is 65536 bytes.
public func parseRequest(fd: Int32) -> Result[HttpRequest, Error] {
    // Read bytes until we find \r\n\r\n (13, 10, 13, 10)
    var buf = Array[UInt8]();
    var chunk = Array[UInt8](capacity: 4096);
    var i: Int64 = 0;
    while i < 4096 {
        chunk.append(0);
        i = i + 1
    }

    var headerEnd: Int64 = -1;

    loop {
        let slice = Slice(pointer: chunk.asPointer(), count: 4096);
        let n = recv(fd, slice.pointer, slice.count, 0);
        if n <= 0 {
            return .Err(invalidInput())
        }

        var j: Int64 = 0;
        while j < n {
            buf.append(chunk(unchecked: j));
            j = j + 1
        }

        // Check for header end
        headerEnd = findHeaderEnd(buf);
        if headerEnd >= 0 {
            break
        }

        // Limit header size to 64KB
        if buf.count > 65536 {
            return .Err(invalidInput())
        }
    }

    // Convert header bytes to string
    let headerStr = bytesToString(buf, from: 0, to: headerEnd);

    // Split into lines by \r\n
    var lines = headerStr.split("\r\n");

    // Parse request line (first line): METHOD PATH VERSION
    let requestLine = match lines.next() {
        .Some(line) => line,
        .None => return .Err(invalidInput())
    };

    var parts = requestLine.split(" ");
    let methodStr = match parts.next() {
        .Some(s) => s,
        .None => return .Err(invalidInput())
    };
    let rawPath = match parts.next() {
        .Some(s) => s,
        .None => return .Err(invalidInput())
    };
    // version is ignored but consumed

    // Parse method
    let method = match parseMethod(methodStr) {
        .Some(m) => m,
        .None => return .Err(invalidInput())
    };

    // Split path on "?" for query string
    var path = rawPath;
    var queryString = String();
    match rawPath.find("?") {
        .Some(qIdx) => {
            path = rawPath.substringBytes(from: 0, to: qIdx);
            queryString = rawPath.substringBytes(from: qIdx + 1, to: rawPath.byteCount)
        },
        .None => {}
    }

    // Parse headers
    var headers = Array[(String, String)]();
    while let .Some(line) = lines.next() {
        if line.byteCount == 0 {
            break
        }
        match line.find(":") {
            .Some(colonIdx) => {
                let rawName = line.substringBytes(from: 0, to: colonIdx);
                let name = rawName.trimmed();
                let rawValue = line.substringBytes(from: colonIdx + 1, to: line.byteCount);
                let value = rawValue.trimmed();
                headers.append((name, value));
            },
            .None => {}
        }
    }

    // Read body if Content-Length is present
    var body = String();
    match findHeader(headers, "Content-Length") {
        .Some(clStr) => {
            let contentLength = parseDecimal(clStr);
            if contentLength > 0 {
                // We may already have body bytes after the header end
                let bodyStart = headerEnd + 4; // skip \r\n\r\n
                let alreadyRead = buf.count - bodyStart;
                var bodyBytes = Array[UInt8]();

                // Copy body bytes we already have
                var k: Int64 = bodyStart;
                while k < buf.count {
                    bodyBytes.append(buf(unchecked: k));
                    k = k + 1
                }

                // Read remaining body bytes
                while bodyBytes.count < contentLength {
                    let remaining = contentLength - bodyBytes.count;
                    var readSize: Int64 = 4096;
                    if remaining < readSize {
                        readSize = remaining
                    }
                    var bodyChunk = Array[UInt8](capacity: readSize);
                    var bi: Int64 = 0;
                    while bi < readSize {
                        bodyChunk.append(0);
                        bi = bi + 1
                    }
                    let bodySlice = Slice(pointer: bodyChunk.asPointer(), count: readSize);
                    let bn = recv(fd, bodySlice.pointer, bodySlice.count, 0);
                    if bn <= 0 {
                        break
                    }
                    var bj: Int64 = 0;
                    while bj < bn {
                        bodyBytes.append(bodyChunk(unchecked: bj));
                        bj = bj + 1
                    }
                }

                body = bytesToString(bodyBytes, from: 0, to: bodyBytes.count)
            }
        },
        .None => {}
    }

    .Ok(HttpRequest(
        method: method,
        path: path,
        headers: headers,
        body: body,
        queryString: queryString
    ))
}

// ============================================================================
// HELPERS
// ============================================================================

/// Looks up a header value by name in the headers array.
func findHeader(headers: Array[(String, String)], name: String) -> String? {
    var i: Int64 = 0;
    while i < headers.count {
        let pair = headers(unchecked: i);
        if pair.0.equals(name) {
            return .Some(pair.1)
        }
        i = i + 1
    }
    .None
}

/// Scans a byte buffer for the \r\n\r\n sequence (bytes 13, 10, 13, 10).
/// Returns the byte offset of the first \r, or -1 if not found.
func findHeaderEnd(buf: Array[UInt8]) -> Int64 {
    let bufLen = buf.count;
    if bufLen < 4 {
        return -1
    }
    var i: Int64 = 0;
    let limit = bufLen - 3;
    while i < limit {
        if buf(unchecked: i) == 13 and buf(unchecked: i + 1) == 10 and buf(unchecked: i + 2) == 13 and buf(unchecked: i + 3) == 10 {
            return i
        }
        i = i + 1
    };
    return -1
}

/// Converts a range of bytes in an array to a String.
func bytesToString(buf: Array[UInt8], from start: Int64, to end: Int64) -> String {
    var result = String();
    var i = start;
    while i < end {
        result.appendByte(buf(unchecked: i));
        i = i + 1
    }
    result
}

/// Parses a decimal integer string into an Int64.
/// Returns 0 for invalid input.
func parseDecimal(s: String) -> Int64 {
    var result: Int64 = 0;
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let byte = s.byteAtUnchecked(i);
        let digit = Int64(from: byte) - 48; // '0' = 48
        if digit >= 0 and digit <= 9 {
            result = result * 10 + digit
        }
        i = i + 1
    }
    result
}
