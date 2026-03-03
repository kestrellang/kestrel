// HTTP request wire parsing
//
// Reads raw bytes from a socket and produces a Request.

module perch.parse

import http.method.(HttpMethod, parseMethod)
import http.headers.(Headers)
import http.url.(parseUrl, ParsedUrl)
import http.wire.(findHeaderEnd, bytesToString, parseDecimal)
import perch.request.(Request)

/// Reads and parses an HTTP request from a raw socket fd.
///
/// Reads bytes until the header terminator (\r\n\r\n) is found,
/// then parses the request line, headers, and optional body.
/// Maximum header size is 65536 bytes.
public func parseHttpRequest(fd: Int32) -> Result[Request, Error] {
    // Read bytes until we find \r\n\r\n
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

        headerEnd = findHeaderEnd(buf);
        if headerEnd >= 0 {
            break
        }

        if buf.count > 65536 {
            return .Err(invalidInput())
        }
    }

    // Convert header bytes to string
    let headerStr = bytesToString(buf, from: 0, to: headerEnd);

    // Split into lines by \r\n
    var lines = headerStr.split("\r\n");

    // Parse request line: METHOD PATH VERSION
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

    let method = match parseMethod(methodStr) {
        .Some(m) => m,
        .None => return .Err(invalidInput())
    };

    // Parse URL
    let parsed = parseUrl(rawPath);

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

    // Read body if Content-Length is present
    var body = String();
    match headers.value(forName: "Content-Length") {
        .Some(clStr) => {
            let contentLength = parseDecimal(clStr);
            if contentLength > 0 {
                let bodyStart = headerEnd + 4;
                let alreadyRead = buf.count - bodyStart;
                var bodyBytes = Array[UInt8]();

                var k: Int64 = bodyStart;
                while k < buf.count {
                    bodyBytes.append(buf(unchecked: k));
                    k = k + 1
                }

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

