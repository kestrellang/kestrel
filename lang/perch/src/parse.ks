/// HTTP request wire parsing — reads raw bytes from a socket and
/// produces a `Request`.

module perch.parse

import http.method.(HttpMethod, parseMethod)
import http.headers.(Headers)
import http.url.(parseUrl, ParsedUrl, parseQueryString)
import http.wire.(findHeaderEnd, parseDecimal)
import http.cookie.(parseCookieHeader)
import perch.request.(Request)
import std.io.error.(IoError)

/// Reads and parses an HTTP request from a raw socket file descriptor.
///
/// Reads bytes until the header terminator (`\r\n\r\n`) is found,
/// then parses the request line, headers, and optional body.
///
/// # Errors
///
/// Returns `IoError` when:
/// - The socket returns zero bytes (client disconnected)
/// - The header block exceeds 65536 bytes
/// - The request line is malformed (missing method, path, or unrecognized method)
///
/// # Examples
///
/// ```
/// let request = try parseHttpRequest(socketFd);
/// println(request.method.toString() + " " + request.path);
/// ```
public func parseHttpRequest(fileDescriptor: Int32) -> Result[Request, IoError] {
    var buffer = Array[UInt8]();
    // Uninitialized 4 KiB scratch buffer — recv overwrites it, so zero-filling
    // (`repeating: 0`) would just be a wasted 4096-element write per request.
    var chunk = Array[UInt8](capacity: 4096);

    var headerEnd: Int64 = -1;

    loop {
        let bytesRead = recv(fileDescriptor, chunk.asPointer(), 4096, 0);
        if bytesRead <= 0 {
            return .Err(invalidInput())
        }

        // Bulk-append exactly the bytes recv produced, in one pass:
        // append(contentsOf:) grows once and copies once → O(n).
        buffer.append(contentsOf: ArraySlice(pointer: chunk.asPointer(), count: bytesRead));

        headerEnd = findHeaderEnd(buffer);
        if headerEnd >= 0 {
            break
        }

        if buffer.count > 65536 {
            return .Err(invalidInput())
        }
    }

    let headerStr = String(fromUtf8: buffer.asSlice()(0..<headerEnd)) ?? String();

    guard let .Some(firstLineEnd) = headerStr.firstIndex(of: "\r\n") else {
        return .Err(invalidInput());
    }
    let headerSlice = headerStr.asSlice();
    let requestLine = headerSlice.subslice(from: headerSlice.start, to: firstLineEnd.value).toOwned();

    var requestLineParts = requestLine.split(" ").iter();
    guard let .Some(methodSlice) = requestLineParts.next() else { return .Err(invalidInput()); }
    guard let .Some(rawPathSlice) = requestLineParts.next() else { return .Err(invalidInput()); }
    guard let .Some(method) = parseMethod(methodSlice.toOwned()) else { return .Err(invalidInput()); }

    let parsed = parseUrl(rawPathSlice.toOwned());

    let headerLines = headerSlice.subslice(from: firstLineEnd.value + 2, to: headerSlice.end).toOwned();
    let headers = Headers.parse(from: headerLines);

    var body = String();
    if let .Some(lengthStr) = headers.value(forName: "Content-Length") {
        let contentLength = parseDecimal(lengthStr);
        if contentLength > 0 {
            let bodyStart = headerEnd + 4;
            var bodyBytes = Array[UInt8]();

            bodyBytes.append(contentsOf: buffer.asSlice()(bodyStart..<buffer.count));

            while bodyBytes.count < contentLength {
                let remaining = contentLength - bodyBytes.count;
                var readSize: Int64 = 4096;
                if remaining < readSize {
                    readSize = remaining
                }
                var bodyChunk = Array[UInt8](capacity: readSize);
                let bytesRead = recv(fileDescriptor, bodyChunk.asPointer(), readSize, 0);
                if bytesRead <= 0 {
                    break
                }
                bodyBytes.append(contentsOf: ArraySlice(pointer: bodyChunk.asPointer(), count: bytesRead));
            }

            body = String(fromUtf8: bodyBytes.asSlice()) ?? String()
        }
    }

    let parsedQueryParams = parseQueryString(parsed.queryString);
    let parsedCookies = match headers.value(forName: "Cookie") {
        .Some(cookieHeader) => parseCookieHeader(cookieHeader),
        .None => Array[(String, String)]()
    };

    .Ok(Request(
        method: method,
        path: parsed.path,
        segments: parsed.segments,
        queryString: parsed.queryString,
        headers: headers,
        body: body,
        pathParams: Dictionary[String, String](),
        store: Dictionary[String, String](),
        queryParams: parsedQueryParams,
        cookies: parsedCookies
    ))
}

