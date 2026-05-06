/// HTTP response returned by Swoop requests.

module swoop.response

import http.status.(StatusCode)
import http.headers.(Headers)
import quill.value.(Value)
import quill.error.(DeserializeError)
import quill.deserialize.(Deserialize)
import swoop.error.(SwoopError)

// ============================================================================
// RESPONSE
// ============================================================================

/// An HTTP response with status, headers, and body (as both string and bytes).
///
/// # Examples
///
/// ```
/// let res = try Swoop().fetch("http://example.com");
/// if res.status.isSuccess() {
///     println(res.body);
/// }
/// ```
public struct Response: Cloneable {
    public var status: StatusCode
    public var headers: Headers
    public var body: String
    public var bodyBytes: Array[UInt8]

    public init(status: StatusCode, headers: Headers, body: String, bodyBytes: Array[UInt8]) {
        self.status = status;
        self.headers = headers;
        self.body = body;
        self.bodyBytes = bodyBytes;
    }

    public func clone() -> Response {
        Response(self.status.clone(), self.headers.clone(), self.body.clone(), self.bodyBytes.clone())
    }
}

// ============================================================================
// DESERIALIZATION
// ============================================================================

extend Response {
    /// Parses the response body as a JSON `Value`.
    ///
    /// # Examples
    ///
    /// ```
    /// let res = try Swoop().fetch("http://api.example.com/data");
    /// let json = try res.json();
    /// ```
    public func json() -> Result[Value, DeserializeError] = quill.json.Json.decode(self.body)
}

// ============================================================================
// VALIDATION
// ============================================================================

extend Response {
    /// Returns the response if the status is 2xx, or a `SwoopError`
    /// containing the status code otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let res = try Swoop().fetch(url);
    /// let validated = try res.validate();
    /// ```
    public func validate() -> Result[Response, SwoopError] {
        if self.status.isSuccess() {
            .Ok(self)
        } else {
            .Err(SwoopError.httpError(self.status.code))
        }
    }
}
