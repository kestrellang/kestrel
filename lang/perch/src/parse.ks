/// HTTP request wire parsing — reads raw bytes from a stream and
/// produces a `Request`.

module perch

import http.method.(HttpMethod, parseMethod)
import http.headers.(Headers)
import http.wire.(findHeaderEnd, parseDecimal)
import std.streams.(IoError, Readable)

/// Reads and parses an HTTP request from a connection.
///
/// Reads bytes until the header terminator (`\r\n\r\n`) is found,
/// then parses the request line, headers, and optional body.
///
/// # Errors
///
/// Returns `IoError` when:
/// - The stream returns zero bytes (client disconnected)
/// - The header block exceeds 65536 bytes
/// - The request line is malformed (missing method, path, or unrecognized method)
///
/// # Examples
///
/// ```
/// let request = try parseHttpRequest(stream);
/// println(request.method.toString() + " " + request.path);
/// ```
public func parseHttpRequest[S](mutating stream: S) -> Request throws IoError where S: Readable, S: not Copyable, S: not Static {
    var buffer = [];
    var chunk = Array[UInt8](repeating: 0, count: 4096);

    var headerEnd: Int64 = -1;

    loop {
        let bytesRead = try stream.read(into: chunk.asMutableSlice());
        if bytesRead <= 0 {
            throw IoError.invalidInput
        }

        // Bulk-append exactly the bytes read, in one pass:
        // append(contentsOf:) grows once and copies once → O(n).
        buffer.append(contentsOf: chunk(0..<bytesRead));

        headerEnd = findHeaderEnd(buffer);
        if headerEnd >= 0 {
            break
        }

        if buffer.count > 65536 {
            throw IoError.invalidInput
        }
    }

    let headerStr = String(fromUtf8: buffer(0..<headerEnd)) ?? String();

    guard let .Some(firstLineEnd) = headerStr.firstIndex(of: "\r\n") else {
        throw IoError.invalidInput;
    }
    let headerSlice = headerStr.asSlice();
    let requestLine = headerSlice.subslice(from: 0, to: firstLineEnd.value).toOwned();

    var requestLineParts = requestLine.split(by: " ").iter();
    guard let .Some(methodSlice) = requestLineParts.next() else { throw IoError.invalidInput; }
    guard let .Some(rawPathSlice) = requestLineParts.next() else { throw IoError.invalidInput; }
    guard let .Some(method) = parseMethod(methodSlice.toOwned()) else { throw IoError.invalidInput; }

    let headerLines = headerSlice.subslice(from: firstLineEnd.value + 2, to: headerSlice.bytes.count).toOwned();
    let headers = Headers.parse(from: headerLines);

    var body = String();
    if let .Some(lengthStr) = headers.value(forName: "Content-Length") {
        let contentLength = parseDecimal(lengthStr);
        if contentLength > 0 {
            let bodyStart = headerEnd + 4;
            var bodyBytes = [];

            bodyBytes.append(contentsOf: buffer(bodyStart..<buffer.count));

            while bodyBytes.count < contentLength {
                let remaining = contentLength - bodyBytes.count;
                var readSize: Int64 = 4096;
                if remaining < readSize {
                    readSize = remaining
                }
                var bodyChunk = Array[UInt8](repeating: 0, count: readSize);
                let bytesRead = match stream.read(into: bodyChunk.asMutableSlice()) {
                    .Ok(n) => n,
                    .Err(_) => 0
                };
                if bytesRead <= 0 {
                    break
                }
                bodyBytes.append(contentsOf: bodyChunk(0..<bytesRead));
            }

            body = String(fromUtf8: bodyBytes.asSlice()) ?? String()
        }
    }

    .Ok(Request(method, url: rawPathSlice.toOwned(), headers: headers, body: body))
}

