/// HTTP request wire parsing — reads raw bytes from a socket and
/// produces a `Request`.

module perch.parse

import http.method.(HttpMethod, parseMethod)
import http.headers.(Headers)
import http.url.(parseUrl, ParsedUrl)
import http.wire.(findHeaderEnd, bytesToString, parseDecimal)
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
    var chunk = Array[UInt8](repeating: 0, count: 4096);

    var headerEnd: Int64 = -1;

    loop {
        let slice = Slice(pointer: chunk.asPointer(), count: 4096);
        let bytesRead = recv(fileDescriptor, slice.pointer, slice.count, 0);
        if bytesRead <= 0 {
            return .Err(invalidInput())
        }

        for j in 0..<bytesRead {
            buffer.append(chunk(unchecked: j))
        }

        headerEnd = findHeaderEnd(buffer);
        if headerEnd >= 0 {
            break
        }

        if buffer.count > 65536 {
            return .Err(invalidInput())
        }
    }

    let headerStr = bytesToString(buffer, from: 0, to: headerEnd);

    guard let .Some(firstLineEnd) = headerStr.find("\r\n") else {
        return .Err(invalidInput());
    }
    let requestLine = headerStr.substringBytes(from: 0, to: firstLineEnd);

    var parts = requestLine.split(" ");
    guard let .Some(methodStr) = parts.next() else { return .Err(invalidInput()); }
    guard let .Some(rawPath) = parts.next() else { return .Err(invalidInput()); }
    guard let .Some(method) = parseMethod(methodStr) else { return .Err(invalidInput()); }

    let parsed = parseUrl(rawPath);

    let headerLines = headerStr.substringBytes(from: firstLineEnd + 2, to: headerStr.byteCount);
    let headers = Headers.parse(from: headerLines);

    var body = String();
    if let .Some(lengthStr) = headers.value(forName: "Content-Length") {
        let contentLength = parseDecimal(lengthStr);
        if contentLength > 0 {
            let bodyStart = headerEnd + 4;
            var bodyBytes = Array[UInt8]();

            for k in bodyStart..<buffer.count {
                bodyBytes.append(buffer(unchecked: k))
            }

            while bodyBytes.count < contentLength {
                let remaining = contentLength - bodyBytes.count;
                var readSize: Int64 = 4096;
                if remaining < readSize {
                    readSize = remaining
                }
                var bodyChunk = Array[UInt8](repeating: 0, count: readSize);
                let bodySlice = Slice(pointer: bodyChunk.asPointer(), count: readSize);
                let bytesRead = recv(fileDescriptor, bodySlice.pointer, bodySlice.count, 0);
                if bytesRead <= 0 {
                    break
                }
                for copyIdx in 0..<bytesRead {
                    bodyBytes.append(bodyChunk(unchecked: copyIdx))
                }
            }

            body = bytesToString(bodyBytes, from: 0, to: bodyBytes.count)
        }
    }

    .Ok(Request(
        method: method,
        path: parsed.path,
        segments: parsed.segments,
        queryString: parsed.queryString,
        headers: headers,
        body: body,
        pathParams: Dictionary[String, String](),
        store: Dictionary[String, String]()
    ))
}

