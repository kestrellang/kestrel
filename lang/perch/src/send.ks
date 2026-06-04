/// HTTP response wire serialization.

module perch.send

import perch.response.(Response)
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
/// let response = Response.ok(Text("hello"));
/// let _ = sendResponse(response, to: socketFd);
/// ```
public func sendResponse(response: Response, to fileDescriptor: Int32) -> Result[(), IoError] {
    var resp = String(capacity: 256 + response.bodyContent.byteCount);

    resp.append("HTTP/1.1 \(response.status.code) \(response.status.text())\r\n");

    resp.append(response.headers.toWireFormat());

    resp.append("Content-Length: \(response.bodyContent.byteCount)\r\n");

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

    // Write the response string's UTF-8 bytes straight to the socket.
    // asByteSlice() is a non-owning view over `content`'s live buffer —
    // no toBytes() array allocation, no COW clone, no per-byte copy.
    // `content` outlives this loop, so the pointer stays valid.
    let bytes = content.asByteSlice();

    var sent: Int64 = 0;
    while sent < length {
        let ptr = bytes.pointer.offset(by: sent);
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
