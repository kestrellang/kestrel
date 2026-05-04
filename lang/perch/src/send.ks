/// HTTP response wire serialization.

module perch.send

import perch.response.(Response)
import http.wire.(stringToBytes)
import std.io.error.(IoError)

/// Serializes and sends an HTTP response over a socket file descriptor.
///
/// Builds the full HTTP/1.1 wire format — status line, headers
/// (including Content-Length and Connection: close), and body — then
/// writes it to the socket in a single pass.
///
/// # Errors
///
/// Returns `IoError` when the socket write fails (broken pipe, connection
/// reset) or a partial write cannot be completed.
///
/// # Examples
///
/// ```
/// let response = Response.ok(text: "hello");
/// let _ = sendResponse(response, to: socketFd);
/// ```
public func sendResponse(response: Response, to fileDescriptor: Int32) -> Result[(), IoError] {
    var resp = String(capacity: 256 + response.bodyContent.byteCount);

    resp.append("HTTP/1.1 ");
    resp.append(response.status.code.format());
    resp.append(" ");
    resp.append(response.status.text());
    resp.append("\r\n");

    resp.append(response.headers.toWireFormat());

    resp.append("Content-Length: ");
    resp.append(response.bodyContent.byteCount.format());
    resp.append("\r\n");

    resp.append("Connection: close");
    resp.append("\r\n");

    resp.append("\r\n");

    resp.append(response.bodyContent);

    sendAllBytes(fileDescriptor, resp)
}

/// Writes all bytes of a string to a socket, retrying on partial writes.
func sendAllBytes(fileDescriptor: Int32, content: String) -> Result[(), IoError] {
    let length = content.byteCount;
    if length == 0 {
        return .Ok(())
    }

    let buffer = stringToBytes(content);

    var sent: Int64 = 0;
    while sent < length {
        let ptr = buffer.asPointer().offset(by: sent);
        let remaining = length - sent;
        let bytesWritten = send(fileDescriptor, ptr, remaining, 0);
        if bytesWritten < 0 {
            return .Err(IoError.last())
        }
        if bytesWritten == 0 {
            return .Err(IoError(code: 32))
        }
        sent = sent + bytesWritten
    }
    .Ok(())
}
