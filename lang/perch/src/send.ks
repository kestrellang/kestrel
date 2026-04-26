// HTTP response wire serialization

module perch.send

import perch.response.(Response)

/// Serializes and sends an HTTP response over a socket fd.
///
/// Sends the status line, headers (including Content-Length and
/// Connection: close), and body.
public func sendResponse(response: Response, to fd: Int32) -> Result[(), Error] {
    var resp = String(capacity: 256 + response.bodyContent.byteCount);

    // Status line: HTTP/1.1 200 OK\r\n
    resp.append("HTTP/1.1 ");
    resp.append(response.status.code.format());
    resp.append(" ");
    resp.append(response.status.text());
    resp.append("\r\n");

    var i: Int64 = 0;
    while i < response.headers.entries.count {
        let pair = response.headers.entries(unchecked: i);
        resp.append(pair.0);
        resp.append(": ");
        resp.append(pair.1);
        resp.append("\r\n");
        i = i + 1
    }

    // Content-Length
    resp.append("Content-Length: ");
    resp.append(response.bodyContent.byteCount.format());
    resp.append("\r\n");

    resp.append("Connection: close");
    resp.append("\r\n");

    resp.append("\r\n");

    // Body
    resp.append(response.bodyContent);

    // Send all bytes
    sendAllBytes(fd, resp)
}

/// Sends all bytes of a string over a socket fd, retrying partial writes.
func sendAllBytes(fd: Int32, s: String) -> Result[(), Error] {
    let len = s.byteCount;
    if len == 0 {
        return .Ok(())
    }

    var buf = Array[UInt8](capacity: len);
    var i: Int64 = 0;
    while i < len {
        buf.append(s.bytes(unchecked: i));
        i = i + 1
    }

    var sent: Int64 = 0;
    while sent < len {
        let ptr = buf.asPointer().offset(by: sent);
        let remaining = len - sent;
        let n = send(fd, ptr, remaining, 0);
        if n < 0 {
            return .Err(Error.last())
        }
        if n == 0 {
            return .Err(Error(32))
        }
        sent = sent + n
    }
    .Ok(())
}
