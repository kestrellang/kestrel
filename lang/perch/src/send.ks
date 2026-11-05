/// HTTP response wire serialization.

module perch

import std.io.error.(IoError)

/// Serializes and sends an HTTP response over a connection.
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
///  sendResponse(response, to: stream);
/// ```
func sendResponse(response: Response, to fileDescriptor: Int32) -> () throws IoError {
    var resp = String(capacity: 256 + response.bodyContent.bytes.count);

    resp.append("HTTP/1.1 \(response.status.code) \(response.status.text())\r\n");

    resp.append(response.headers.toWireFormat());

    resp.append("Content-Length: \(response.bodyContent.bytes.count)\r\n");

    resp.append("Connection: close");
    resp.append("\r\n");

    resp.append("\r\n");

    resp.append(response.bodyContent);

    stream.write(text: resp)
}
