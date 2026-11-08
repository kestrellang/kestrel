/// HTTP/1.1 wire protocol — serializes requests and parses responses.
///
/// The reverse of perch: perch parses requests and serializes responses;
/// swoop serializes requests and parses responses.

module swoop.send

import http.method.(HttpMethod)
import http.status.(StatusCode)
import http.headers.(Headers)
import swoop.error.(SwoopError)
import std.streams.(IoError, Readable, Writable)
import swoop.response.(Response)
import http.wire.(findHeaderEnd, parseDecimal, dechunk, stringToBytes)
import swoop.url.(ClientUrl)
import http.content.(Content)

// ============================================================================
// SEND REQUEST (no content)
// ============================================================================

/// Sends an HTTP request without a body and reads the response.
public func sendRequest[S](
    mutating stream: S,
    method: HttpMethod,
    url: ClientUrl,
    headers: Headers
) -> Response throws SwoopError where S: Readable, S: Writable, S: not Copyable, S: not Static {
    var req = String();

    req.append(method.toString());
    req.append(" ");
    req.append(url.requestPath());
    req.append(" HTTP/1.1\r\n");

    req.append("Host: ");
    req.append(url.hostHeader());
    req.append("\r\n");

    req.append(headers.toWireFormat());

    req.append("Connection: close\r\n");
    req.append("\r\n");

    match sendAllString(stream, req) {
        .Ok(_) => {},
        .Err(e) => throw SwoopError.connectionFailed("failed to send request")
    }

    readResponse(stream)
}

// ============================================================================
// SEND REQUEST (with content)
// ============================================================================

/// Sends an HTTP request with a body and reads the response.
public func sendRequest[S, C](
    mutating stream: S,
    method: HttpMethod,
    url: ClientUrl,
    headers: Headers,
    content: C
) -> Response throws SwoopError where S: Readable, S: Writable, S: not Copyable, S: not Static, C: Content {
    var req = String();

    req.append(method.toString());
    req.append(" ");
    req.append(url.requestPath());
    req.append(" HTTP/1.1\r\n");

    req.append("Host: ");
    req.append(url.hostHeader());
    req.append("\r\n");

    req.append(headers.toWireFormat());

    let contentBytes = content.toBytes();

    if not headers.has("Content-Type") {
        if let .Some(ct) = content.contentType() {
            req.append("Content-Type: ");
            req.append(ct);
            req.append("\r\n")
        }
    }

    req.append("Content-Length: \(contentBytes.count)\r\n");

    req.append("Connection: close\r\n");
    req.append("\r\n");

    match sendAllString(stream, req) {
        .Ok(_) => {},
        .Err(e) => throw SwoopError.connectionFailed("failed to send request")
    }

    if contentBytes.count > 0 {
        match sendAllBytes(stream, contentBytes) {
            .Ok(_) => {},
            .Err(e) => throw SwoopError.connectionFailed("failed to send body")
        }
    }

    readResponse(stream)
}

// ============================================================================
// READ RESPONSE
// ============================================================================

/// Reads an HTTP response from a stream.
func readResponse[S](mutating stream: S) -> Response throws SwoopError where S: Readable, S: not Copyable, S: not Static {

    var buf = [];
    var chunk = Array[UInt8](repeating: 0, count: 4096);

    var headerEnd: Int64 = -1;

    loop {
        let n = match stream.read(into: chunk(..<4096)) {
            .Ok(bytes) => bytes,
            .Err(_) => throw SwoopError.connectionFailed("failed to read response")
        };
        if n <= 0 {
            if buf.count == 0 {
                throw SwoopError.connectionFailed("connection closed")
            }
            break
        }

        let readSlice = &chunk(0..<n);
        buf.append(contentsOf: readSlice);

        headerEnd = findHeaderEnd(buf);
        if headerEnd >= 0 {
            break
        }

        if buf.count > 65536 {
            throw SwoopError.invalidResponse("headers too large")
        }
    }

    if headerEnd < 0 {
        throw SwoopError.invalidResponse("no header terminator found")
    }

    let headerStr = String(fromUtf8: buf(0..<headerEnd)) ?? String();

    let hdrSlice = headerStr.asSlice();
    guard let .Some(firstLineEnd) = headerStr.firstIndex(of: "\r\n") else {
        throw SwoopError.invalidResponse("empty response");
    }
    let statusLine = hdrSlice.subslice(from: hdrSlice.start, to: firstLineEnd.value).toOwned();

    let slSlice = statusLine.asSlice();
    guard let .Some(spaceIdx) = statusLine.firstIndex(of: " ") else {
        throw SwoopError.invalidResponse("malformed status line");
    }
    let afterVersion = slSlice.subslice(from: spaceIdx.value + 1, to: slSlice.end).toOwned();
    let avSlice = afterVersion.asSlice();
    let statusCode = match afterVersion.firstIndex(of: " ") {
        .Some(sp2) => parseDecimal(avSlice.subslice(from: avSlice.start, to: sp2.value).toOwned()),
        .None => parseDecimal(afterVersion)
    };

    let headerLines = hdrSlice.subslice(from: firstLineEnd.value + 2, to: hdrSlice.end).toOwned();
    let headers = Headers.parse(from: headerLines);

    let bodyStart = headerEnd + 4;
    var rawBuf = [];

    let initialBody = &buf(bodyStart..<buf.count);
    rawBuf.append(contentsOf: initialBody);

    let isChunked = match headers.value(forName: "Transfer-Encoding") {
        .Some(te) => te.contains("chunked"),
        .None => false
    };

    if let .Some(clStr) = headers.value(forName: "Content-Length") {
        let contentLength = parseDecimal(clStr);
        while rawBuf.count < contentLength {
            let remaining = contentLength - rawBuf.count;
            var readSize: Int64 = 4096;
            if remaining < readSize {
                readSize = remaining
            }
            var bodyChunk = Array[UInt8](repeating: 0, count: readSize);
            let bn = match stream.read(into: bodyChunk(..<readSize)) {
                .Ok(bytes) => bytes,
                .Err(_) => break
            };
            if bn <= 0 {
                break
            }
            let chunkSlice = &bodyChunk(0..<bn);
            rawBuf.append(contentsOf: chunkSlice);
        }
    } else {
        loop {
            var readChunk = Array[UInt8](repeating: 0, count: 4096);
            let rn = match stream.read(into: readChunk(..<4096)) {
                .Ok(bytes) => bytes,
                .Err(_) => break
            };
            if rn <= 0 {
                break
            }
            let chunkSlice = &readChunk(0..<rn);
            rawBuf.append(contentsOf: chunkSlice);
        }
    }

    var bodyBuf = if isChunked {
        dechunk(rawBuf)
    } else {
        rawBuf
    };

    let bodyStr = String(fromUtf8: bodyBuf.asSlice()) ?? String();

    .Ok(Response(StatusCode(statusCode), headers, bodyStr, bodyBuf))
}

// ============================================================================
// HELPERS
// ============================================================================

/// Sends all bytes of a string over a stream.
func sendAllString[S](mutating stream: S, s: String) -> () throws IoError where S: Writable, S: not Copyable, S: not Static {
    stream.write(text: s)
}

/// Sends all bytes of a buffer over a stream.
func sendAllBytes[S](mutating stream: S, buf: [UInt8]) -> () throws IoError where S: Writable, S: not Copyable, S: not Static {
    stream.writeAll(from: buf.asSlice())
}
