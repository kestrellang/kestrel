// HTTP response type

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

/// An HTTP response returned by Swoop.
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
    /// Parses the response body as a JSON Value.
    public func json() -> Result[Value, DeserializeError] {
        quill.json.Json.decode(self.body)
    }
}

// ============================================================================
// VALIDATION
// ============================================================================

extend Response {
    /// Returns an error if the status code is not 2xx (success).
    /// On success, returns the response unchanged.
    /// On failure, the SwoopError contains the status code.
    public func validate() -> Result[Response, SwoopError] {
        if self.status.isSuccess() {
            .Ok(self)
        } else {
            .Err(SwoopError.httpError(self.status.code))
        }
    }
}
