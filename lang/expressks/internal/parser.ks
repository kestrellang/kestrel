// HTTP request parsing
//
// This module provides functions to parse raw HTTP request data.

module expressks.internal.parser;

import std.collections.dictionary;
import std.collections.array;
import std.text.string;
import expressks.http.(Request, HttpMethod);
import expressks.internal.path.(parseQueryString, substring, substringFrom, substringTo);

// HTTP parse error
public struct HttpParseError: Error {
    public var message: String;

    public init(message: String) {
        self.message = message;
    }

    public var description: String {
        "HttpParseError: " + self.message
    }
}

// Parse raw HTTP request bytes into Request struct
public func parseHttpRequest(raw: String) -> Result[Request, HttpParseError] {
    // Find header/body separator (\r\n\r\n)
    let headerEnd = findSequence(str: raw, seq: "\r\n\r\n");

    let headerSection = match headerEnd {
        .Some(let pos) => substringTo(str: raw, end: pos),
        .None => raw
    };

    let body = match headerEnd {
        .Some(let pos) => substringFrom(str: raw, start: pos + 4),
        .None => ""
    };

    // Split header section into lines
    var lines = Array[String]();
    for line in headerSection.split(on: "\r\n") {
        lines.append(line);
    }

    if lines.count == 0 {
        return .Err(HttpParseError(message: "Empty request"));
    }

    // Parse request line: "GET /path HTTP/1.1"
    let requestLine = lines(unchecked: 0);
    var requestParts = Array[String]();
    for part in requestLine.split(on: " ") {
        requestParts.append(part);
    }

    if requestParts.count < 2 {
        return .Err(HttpParseError(message: "Invalid request line"));
    }

    let methodStr = requestParts(unchecked: 0);
    let fullPath = requestParts(unchecked: 1);

    // Parse method
    let method = match HttpMethod.parse(methodStr) {
        .Some(let m) => m,
        .None => return .Err(HttpParseError(message: "Unknown HTTP method: " + methodStr))
    };

    // Parse path and query string
    let queryPos = fullPath.find(substring: "?");
    let (path, query) = match queryPos {
        .Some(let pos) => {
            let p = substringTo(str: fullPath, end: pos);
            let q = substringFrom(str: fullPath, start: pos + 1);
            (p, parseQueryString(q))
        },
        .None => (fullPath, Dictionary[String, String]())
    };

    // Parse headers
    var headers = Dictionary[String, String]();
    for i in 1..<lines.count {
        let line = lines(unchecked: i);
        if line.isEmpty {
            continue;
        }

        let colonPos = line.find(substring: ":");
        match colonPos {
            .Some(let pos) => {
                let name = substringTo(str: line, end: pos).trim().lowercase();
                let value = substringFrom(str: line, start: pos + 1).trim();
                headers.insert(value: value, for: name);
            },
            .None => {
                // Invalid header line, skip
            }
        }
    }

    .Ok(Request(
        method: method,
        path: path,
        headers: headers,
        params: Dictionary[String, String](),  // Filled in by router
        query: query,
        body: body
    ))
}

// Format HTTP response for sending
public func formatHttpResponse(
    statusCode: Int,
    statusMessage: String,
    contentType: String,
    body: String,
    extraHeaders: Dictionary[String, String]
) -> String {
    var result = String();

    // Status line
    result.append(string: "HTTP/1.1 ");
    result.append(string: intToString(statusCode));
    result.append(string: " ");
    result.append(string: statusMessage);
    result.append(string: "\r\n");

    // Content-Type
    if not contentType.isEmpty {
        result.append(string: "Content-Type: ");
        result.append(string: contentType);
        result.append(string: "\r\n");
    }

    // Content-Length
    result.append(string: "Content-Length: ");
    result.append(string: intToString(body.byteCount));
    result.append(string: "\r\n");

    // Extra headers
    for (name, value) in extraHeaders {
        result.append(string: name);
        result.append(string: ": ");
        result.append(string: value);
        result.append(string: "\r\n");
    }

    // Connection: close
    result.append(string: "Connection: close\r\n");

    // End of headers
    result.append(string: "\r\n");

    // Body
    result.append(string: body);

    result
}

// Helper: find a sequence in a string
func findSequence(str: String, seq: String) -> Optional[Int] {
    if seq.isEmpty {
        return .Some(0);
    }
    if seq.byteCount > str.byteCount {
        return .None;
    }

    for i in 0..=(str.byteCount - seq.byteCount) {
        var found = true;
        for j in 0..<seq.byteCount {
            if str.byteAt(index: i + j) != seq.byteAt(index: j) {
                found = false;
                break;
            }
        }
        if found {
            return .Some(i);
        }
    }
    .None
}

// Helper: convert int to string
func intToString(value: Int) -> String {
    if value == 0 {
        return "0";
    }

    var result = String();
    var n = if value < 0 { -value } else { value };
    var digits = Array[UInt8]();

    while n > 0 {
        digits.append(((n % 10) + 48) as UInt8);
        n = n / 10;
    }

    if value < 0 {
        result.append(codePoint: CodePoint(value: 45));  // '-'
    }

    // Reverse digits
    for i in 0..<digits.count {
        let idx = digits.count - 1 - i;
        result.append(codePoint: CodePoint(value: digits(unchecked: idx) as UInt32));
    }

    result
}
