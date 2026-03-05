// HTTP/1.1 wire protocol: serialize requests and parse responses
//
// This is the reverse of what perch does:
//   - perch.parse: parses requests FROM the wire
//   - perch.send:  serializes responses TO the wire
//   - swoop.send:  serializes requests TO the wire, parses responses FROM the wire

module swoop.send

import http.method.(HttpMethod)
import http.status.(StatusCode)
import http.headers.(Headers)
import swoop.error.(SwoopError)
import swoop.response.(Response)
import http.wire.(findHeaderEnd, bytesToString, parseDecimal)
import swoop.url.(ClientUrl)
import swoop.body.(Body)

// ============================================================================
// SEND REQUEST
// ============================================================================

/// Sends an HTTP request over a stream and reads the response.
public func sendRequest[S](
    stream: S,
    method: HttpMethod,
    url: ClientUrl,
    headers: Headers,
    body: Body?
) -> Result[Response, SwoopError] where S: Read, S: Write {
    // Build the request string
    var req = String();

    // Request line: METHOD /path HTTP/1.1\r\n
    req.append(method.toString());
    req.append(" ");
    req.append(url.requestPath());
    req.append(" HTTP/1.1");
    req.append("\r\n");

    req.append("Host: ");
    req.append(url.hostHeader());
    req.append("\r\n");

    // User headers
    var hi: Int64 = 0;
    while hi < headers.entries.count {
        let pair = headers.entries(unchecked: hi);
        req.append(pair.0);
        req.append(": ");
        req.append(pair.1);
        req.append("\r\n");
        hi = hi + 1
    }

    // Body handling
    var bodyBytes = Array[UInt8]();
    match body {
        .Some(b) => {
            bodyBytes = b.toBytes();

            // Set Content-Type if not already set and body implies one
            if not headers.has("Content-Type") {
                match b {
                    .Form(_) => {
                        req.append("Content-Type: application/x-www-form-urlencoded");
                        req.append("\r\n")
                    },
                    _ => {}
                }
            }

            // Content-Length
            req.append("Content-Length: ");
            req.append(bodyBytes.count.format());
            req.append("\r\n")
        },
        .None => {}
    }

    req.append("Connection: close");
    req.append("\r\n");

    req.append("\r\n");

    // Send headers
    var sendStream = stream;
    match sendAllString(sendStream, req) {
        .Ok(_) => {},
        .Err(e) => return .Err(SwoopError.connectionFailed("failed to send request"))
    }

    // Send body
    if bodyBytes.count > 0 {
        match sendAllBytes(sendStream, bodyBytes) {
            .Ok(_) => {},
            .Err(e) => return .Err(SwoopError.connectionFailed("failed to send body"))
        }
    }

    // Read and parse response
    readResponse(sendStream)
}

// ============================================================================
// READ RESPONSE
// ============================================================================

/// Reads an HTTP response from a stream.
func readResponse[S](stream: S) -> Result[Response, SwoopError] where S: Read {
    var recvStream = stream;

    // Read bytes until we find \r\n\r\n (header end)
    var buf = Array[UInt8]();
    var chunk = Array[UInt8](capacity: 4096);
    var ci: Int64 = 0;
    while ci < 4096 {
        chunk.append(0);
        ci = ci + 1
    }

    var headerEnd: Int64 = -1;

    loop {
        let slice = Slice(pointer: chunk.asPointer(), count: 4096);
        let readResult = recvStream.read(into: slice);
        let n = match readResult {
            .Ok(bytes) => bytes,
            .Err(_) => return .Err(SwoopError.connectionFailed("failed to read response"))
        };
        if n <= 0 {
            if buf.count == 0 {
                return .Err(SwoopError.connectionFailed("connection closed"))
            }
            break
        }

        var j: Int64 = 0;
        while j < n {
            buf.append(chunk(unchecked: j));
            j = j + 1
        }

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

    // Parse header section
    let headerStr = bytesToString(buf, from: 0, to: headerEnd);

    // Split into lines
    var lines = headerStr.split("\r\n");

    // Parse status line: HTTP/1.1 200 OK
    let statusLine = match lines.next() {
        .Some(line) => line,
        .None => return .Err(SwoopError.invalidResponse("empty response"))
    };

    // Find first space (after HTTP/1.1)
    let statusCode = match statusLine.find(" ") {
        .Some(spaceIdx) => {
            let afterVersion = statusLine.substringBytes(from: spaceIdx + 1, to: statusLine.byteCount);
            // Find second space (between code and reason)
            match afterVersion.find(" ") {
                .Some(sp2) => {
                    let codeStr = afterVersion.substringBytes(from: 0, to: sp2);
                    parseDecimal(codeStr)
                },
                .None => {
                    // No reason phrase, just status code
                    parseDecimal(afterVersion)
                }
            }
        },
        .None => return .Err(SwoopError.invalidResponse("malformed status line"))
    };

    // Parse headers
    var headers = Headers();
    while let .Some(line) = lines.next() {
        if line.byteCount == 0 {
            break
        }
        match line.find(":") {
            .Some(colonIdx) => {
                let name = line.substringBytes(from: 0, to: colonIdx).trimmed();
                let value = line.substringBytes(from: colonIdx + 1, to: line.byteCount).trimmed();
                headers.add(name, value)
            },
            .None => {}
        }
    }

    // Read body
    let bodyStart = headerEnd + 4;
    var rawBuf = Array[UInt8]();

    // Copy any bytes already read after the header
    var k = bodyStart;
    while k < buf.count {
        rawBuf.append(buf(unchecked: k));
        k = k + 1
    }

    // Check if chunked transfer encoding
    let isChunked = match headers.value(forName: "Transfer-Encoding") {
        .Some(te) => te.contains("chunked"),
        .None => false
    };

    // Read remaining data from stream
    match headers.value(forName: "Content-Length") {
        .Some(clStr) => {
            let contentLength = parseDecimal(clStr);
            while rawBuf.count < contentLength {
                let remaining = contentLength - rawBuf.count;
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
                let readResult = recvStream.read(into: bodySlice);
                let bn = match readResult {
                    .Ok(bytes) => bytes,
                    .Err(_) => break
                };
                if bn <= 0 {
                    break
                }
                var bj: Int64 = 0;
                while bj < bn {
                    rawBuf.append(bodyChunk(unchecked: bj));
                    bj = bj + 1
                }
            }
        },
        .None => {
            // No Content-Length: read until connection closes
            loop {
                var readChunk = Array[UInt8](capacity: 4096);
                var ri: Int64 = 0;
                while ri < 4096 {
                    readChunk.append(0);
                    ri = ri + 1
                }
                let readSlice = Slice(pointer: readChunk.asPointer(), count: 4096);
                let readResult = recvStream.read(into: readSlice);
                let rn = match readResult {
                    .Ok(bytes) => bytes,
                    .Err(_) => break
                };
                if rn <= 0 {
                    break
                }
                var rj: Int64 = 0;
                while rj < rn {
                    rawBuf.append(readChunk(unchecked: rj));
                    rj = rj + 1
                }
            }
        }
    }

    // Dechunk if chunked transfer encoding
    var bodyBuf = if isChunked {
        dechunk(rawBuf)
    } else {
        rawBuf
    };

    let bodyStr = bytesToString(bodyBuf, from: 0, to: bodyBuf.count);

    .Ok(Response(StatusCode(statusCode), headers, bodyStr, bodyBuf))
}

// ============================================================================
// CHUNKED DECODING
// ============================================================================

/// Decodes a chunked transfer-encoded body.
/// Format: hex-size\r\ndata\r\nhex-size\r\ndata\r\n...0\r\n\r\n
func dechunk(raw: Array[UInt8]) -> Array[UInt8] {
    var result = Array[UInt8]();
    var pos: Int64 = 0;
    let len = raw.count;

    loop {
        if pos >= len {
            break
        }

        // Parse hex chunk size
        var chunkSize: Int64 = 0;
        while pos < len {
            let b = raw(unchecked: pos);
            if b == 13 {
                // \r — skip \r\n
                pos = pos + 2;
                break
            }
            // hex digit
            let digit = hexDigitValue(b);
            chunkSize = chunkSize * 16 + digit;
            pos = pos + 1
        }

        // Zero chunk means end
        if chunkSize == 0 {
            break
        }

        // Copy chunk data
        var ci: Int64 = 0;
        while ci < chunkSize and pos < len {
            result.append(raw(unchecked: pos));
            pos = pos + 1;
            ci = ci + 1
        }

        // Skip trailing \r\n after chunk data
        if pos + 1 < len {
            pos = pos + 2
        }
    }

    result
}

/// Returns the numeric value of a hex digit byte.
func hexDigitValue(b: UInt8) -> Int64 {
    let v = Int64(from: b);
    if v >= 48 and v <= 57 { return v - 48 }
    if v >= 65 and v <= 70 { return v - 55 }
    if v >= 97 and v <= 102 { return v - 87 }
    0
}

// ============================================================================
// HELPERS
// ============================================================================

/// Sends all bytes of a string over a stream.
func sendAllString[S](stream: S, s: String) -> Result[(), Error] where S: Write {
    var mutStream = stream;
    let len = s.byteCount;
    if len == 0 {
        return .Ok(())
    }

    var buf = Array[UInt8](capacity: len);
    var i: Int64 = 0;
    while i < len {
        buf.append(s.byteAtUnchecked(i));
        i = i + 1
    }

    sendAllBytes(mutStream, buf)
}

/// Sends all bytes of a buffer over a stream.
func sendAllBytes[S](stream: S, buf: Array[UInt8]) -> Result[(), Error] where S: Write {
    var mutStream = stream;
    let len = buf.count;
    var sent: Int64 = 0;
    while sent < len {
        let ptr = buf.asPointer().offset(by: sent);
        let remaining = len - sent;
        let slice = Slice(pointer: ptr, count: remaining);
        let n = try mutStream.write(from: slice);
        if n == 0 {
            return .Err(Error(32))
        }
        sent = sent + n
    }
    .Ok(())
}
