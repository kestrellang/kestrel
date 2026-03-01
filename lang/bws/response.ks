// HTTP response builder
//
// Provides HttpResponse for constructing and sending HTTP responses.

module bws.response

import std.num.(Int64, Int32, UInt8)
import std.result.(Result)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.net.libc.(send)
import std.io.error.(Error)

// ============================================================================
// HTTP RESPONSE
// ============================================================================

/// An HTTP response builder.
///
/// Build a response by setting status, headers, and body, then send it
/// to a client socket.
///
/// Example:
///     var res = HttpResponse()
///     res.header(name: "X-Custom", value: "hello")
///     res.text("Hello, World!")
///     try res.send(to: clientFd)
public struct HttpResponse {
    public var statusCode: Int64
    public var statusText: String
    public var headers: Array[(String, String)]
    public var bodyContent: String

    /// Creates a new response with 200 OK status.
    public init() {
        self.statusCode = 200;
        self.statusText = "OK";
        self.headers = Array[(String, String)]();
        self.bodyContent = String()
    }

    /// Sets the response status code and text.
    public mutating func setStatus(code: Int64) {
        self.statusCode = code;
        self.statusText = statusTextFor(code)
    }

    /// Adds a response header.
    public mutating func header(name: String, value: String) {
        self.headers.append((name, value))
    }

    /// Sets the body to plain text content.
    /// Also sets Content-Type to text/plain.
    public mutating func text(content: String) {
        self.bodyContent = content;
        self.headers.append(("Content-Type", "text/plain"))
    }

    /// Sets the body to HTML content.
    /// Also sets Content-Type to text/html.
    public mutating func html(content: String) {
        self.bodyContent = content;
        self.headers.append(("Content-Type", "text/html"))
    }

    /// Sets the body to JSON content.
    /// Also sets Content-Type to application/json.
    public mutating func json(content: String) {
        self.bodyContent = content;
        self.headers.append(("Content-Type", "application/json"))
    }

    /// Serializes and sends this response over a socket fd.
    ///
    /// Sends the status line, headers (including Content-Length and
    /// Connection: close), and body.
    public func send(to fd: Int32) -> Result[(), Error] {
        // Build the response string
        var resp = String();

        // Status line: HTTP/1.1 200 OK\r\n
        resp.append("HTTP/1.1 ");
        resp.append(intToString(self.statusCode));
        resp.append(" ");
        resp.append(self.statusText);
        resp.appendByte(13); // \r
        resp.appendByte(10); // \n

        // Headers
        var i: Int64 = 0;
        while i < self.headers.count {
            let pair = self.headers(unchecked: i);
            resp.append(pair.0);
            resp.append(": ");
            resp.append(pair.1);
            resp.appendByte(13);
            resp.appendByte(10);
            i = i + 1
        }

        // Content-Length
        resp.append("Content-Length: ");
        resp.append(intToString(self.bodyContent.byteCount));
        resp.appendByte(13);
        resp.appendByte(10);

        // Connection: close
        resp.append("Connection: close");
        resp.appendByte(13);
        resp.appendByte(10);

        // End of headers
        resp.appendByte(13);
        resp.appendByte(10);

        // Body
        resp.append(self.bodyContent);

        // Send all bytes
        sendAllBytes(fd, resp)
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Maps common HTTP status codes to reason phrases.
public func statusTextFor(code: Int64) -> String {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown"
    }
}

/// Converts an Int64 to its decimal string representation.
func intToString(n: Int64) -> String {
    if n == 0 {
        return "0"
    }

    var result = String();
    var value = n;
    var negative = false;
    if value < 0 {
        negative = true;
        value = 0 - value
    }

    // Build digits in reverse
    var digits = Array[UInt8]();
    while value > 0 {
        let digitValue = value % 10 + 48;
        let digit = UInt8(from: digitValue);
        digits.append(digit);
        value = value / 10
    }

    if negative {
        result.appendByte(45) // '-'
    }

    // Reverse the digits
    var i = digits.count - 1;
    while i >= 0 {
        result.appendByte(digits(unchecked: i));
        i = i - 1
    }

    result
}

/// Sends all bytes of a string over a socket fd.
func sendAllBytes(fd: Int32, s: String) -> Result[(), Error] {
    let len = s.byteCount;
    if len == 0 {
        return .Ok(())
    }

    // Copy string bytes to array for pointer access
    var buf = Array[UInt8](capacity: len);
    var i: Int64 = 0;
    while i < len {
        buf.append(s.byteAtUnchecked(i));
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
            return .Err(Error(32)) // broken pipe
        }
        sent = sent + n
    }
    .Ok(())
}
