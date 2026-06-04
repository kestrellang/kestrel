/// HTTP/1.1 wire protocol — serializes requests and parses responses.
///
/// The reverse of perch: perch parses requests and serializes responses;
/// swoop serializes requests and parses responses.

module swoop.send

import http.method.(HttpMethod)
import http.status.(StatusCode)
import http.headers.(Headers)
import swoop.error.(SwoopError)
import std.io.error.(IoError)
import swoop.response.(Response)
import http.wire.(findHeaderEnd, parseDecimal, dechunk, stringToBytes)
import swoop.url.(ClientUrl)
import http.content.(Content)

// ============================================================================
// SEND REQUEST (no content)
// ============================================================================

/// Sends an HTTP request without a body and reads the response.
public func sendRequest[S](
    stream: S,
    method: HttpMethod,
    url: ClientUrl,
    headers: Headers
) -> Result[Response, SwoopError] where S: Readable, S: Writable {
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

    var sendStream = stream;
    match sendAllString(sendStream, req) {
        .Ok(_) => {},
        .Err(e) => return .Err(SwoopError.connectionFailed("failed to send request"))
    }

    readResponse(sendStream)
}

// ============================================================================
// SEND REQUEST (with content)
// ============================================================================

/// Sends an HTTP request with a body and reads the response.
public func sendRequest[S, C](
    stream: S,
    method: HttpMethod,
    url: ClientUrl,
    headers: Headers,
    content: C
) -> Result[Response, SwoopError] where S: Readable, S: Writable, C: Content {
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

    var sendStream = stream;
    match sendAllString(sendStream, req) {
        .Ok(_) => {},
        .Err(e) => return .Err(SwoopError.connectionFailed("failed to send request"))
    }

    if contentBytes.count > 0 {
        match sendAllBytes(sendStream, contentBytes) {
            .Ok(_) => {},
            .Err(e) => return .Err(SwoopError.connectionFailed("failed to send body"))
        }
    }

    readResponse(sendStream)
}

// ============================================================================
// READ RESPONSE
// ============================================================================

/// Reads an HTTP response from a stream.
func readResponse[S](stream: S) -> Result[Response, SwoopError] where S: Readable {
    var recvStream = stream;

    var buf = Array[UInt8]();
    var chunk = Array[UInt8](repeating: 0, count: 4096);

    var headerEnd: Int64 = -1;

    loop {
        let slice = ArraySlice(pointer: chunk.asPointer(), count: 4096);
        let n = match recvStream.read(into: slice) {
            .Ok(bytes) => bytes,
            .Err(_) => return .Err(SwoopError.connectionFailed("failed to read response"))
        };
        if n <= 0 {
            if buf.count == 0 {
                return .Err(SwoopError.connectionFailed("connection closed"))
            }
            break
        }

        let readSlice = chunk.asSlice()(0..<n);
        buf.append(contentsOf: readSlice);

        headerEnd = findHeaderEnd(buf);
        if headerEnd >= 0 {
            break
        }

        if buf.count > 65536 {
            return .Err(SwoopError.invalidResponse("headers too large"))
        }
    }

    if headerEnd < 0 {
        return .Err(SwoopError.invalidResponse("no header terminator found"))
    }

    let headerStr = String(fromUtf8: buf.asSlice()(0..<headerEnd)) ?? String();

    let hdrSlice = headerStr.asSlice();
    guard let .Some(firstLineEnd) = headerStr.firstIndex(of: "\r\n") else {
        return .Err(SwoopError.invalidResponse("empty response"));
    }
    let statusLine = hdrSlice.subslice(from: hdrSlice.start, to: firstLineEnd.value).toOwned();

    let slSlice = statusLine.asSlice();
    guard let .Some(spaceIdx) = statusLine.firstIndex(of: " ") else {
        return .Err(SwoopError.invalidResponse("malformed status line"));
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
    var rawBuf = Array[UInt8]();

    let initialBody = buf.asSlice()(bodyStart..<buf.count);
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
            let bodySlice = ArraySlice(pointer: bodyChunk.asPointer(), count: readSize);
            let bn = match recvStream.read(into: bodySlice) {
                .Ok(bytes) => bytes,
                .Err(_) => break
            };
            if bn <= 0 {
                break
            }
            let chunkSlice = bodyChunk.asSlice()(0..<bn);
            rawBuf.append(contentsOf: chunkSlice);
        }
    } else {
        loop {
            var readChunk = Array[UInt8](repeating: 0, count: 4096);
            let readSlice = ArraySlice(pointer: readChunk.asPointer(), count: 4096);
            let rn = match recvStream.read(into: readSlice) {
                .Ok(bytes) => bytes,
                .Err(_) => break
            };
            if rn <= 0 {
                break
            }
            let chunkSlice = readChunk.asSlice()(0..<rn);
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
func sendAllString[S](stream: S, s: String) -> Result[(), IoError] where S: Writable {
    var mutStream = stream;
    if s.byteCount == 0 {
        return .Ok(())
    }
    sendAllBytes(mutStream, stringToBytes(s))
}

/// Sends all bytes of a buffer over a stream.
func sendAllBytes[S](stream: S, buf: Array[UInt8]) -> Result[(), IoError] where S: Writable {
    var mutStream = stream;
    let len = buf.count;
    var sent: Int64 = 0;
    while sent < len {
        let ptr = buf.asPointer().offset(by: sent);
        let remaining = len - sent;
        let slice = ArraySlice(pointer: ptr, count: remaining);
        let n = try mutStream.write(from: slice);
        if n == 0 {
            return .Err(IoError(code: 32))
        }
        sent = sent + n
    }
    .Ok(())
}
