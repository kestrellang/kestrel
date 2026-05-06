/// HTTP status codes with reason phrases and class queries.
///
/// `StatusCode` wraps an integer code and provides the standard
/// RFC 9110 reason phrase via `text()`, plus category predicates
/// (`isSuccess`, `isClientError`, etc.). Common codes have static
/// convenience constructors.
///
/// # Examples
///
/// ```
/// let s = StatusCode.ok();
/// s.code;            // 200
/// s.text();          // "OK"
/// s.isSuccess();     // true
///
/// let nf = StatusCode.notFound();
/// nf.isClientError();  // true
/// ```

module http.status

/// An HTTP status code with its standard reason phrase.
///
/// Stores the numeric code as an `Int64`. The `text()` method maps
/// well-known codes to their RFC 9110 reason phrases; unknown codes
/// return `"Unknown"`.
///
/// # Examples
///
/// ```
/// let s = StatusCode(201);
/// s.text();          // "Created"
/// s.isSuccess();     // true
/// s.isRedirect();    // false
/// ```
///
/// # Representation
///
/// A single `Int64` field holding the three-digit HTTP status code.
public struct StatusCode: Cloneable {
    /// The three-digit HTTP status code (e.g. 200, 404, 500).
    public var code: Int64

    /// @name From Code
    /// Creates a status code from an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// StatusCode(404).text();  // "Not Found"
    /// ```
    public init(code: Int64) {
        self.code = code
    }

    /// Returns the standard reason phrase for this status code.
    ///
    /// Covers the most common HTTP/1.1 codes. Returns `"Unknown"` for
    /// codes not in the built-in table.
    ///
    /// # Examples
    ///
    /// ```
    /// StatusCode(200).text();  // "OK"
    /// StatusCode(418).text();  // "Unknown"
    /// ```
    public func text() -> String {
        match self.code {
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
            409 => "Conflict",
            415 => "Unsupported Media Type",
            422 => "Unprocessable Entity",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown"
        }
    }

    /// Returns `true` if the code is in the 2xx (success) range.
    ///
    /// # Examples
    ///
    /// ```
    /// StatusCode(200).isSuccess();  // true
    /// StatusCode(301).isSuccess();  // false
    /// ```
    public func isSuccess() -> Bool { self.code >= 200 and self.code < 300 }

    /// Returns `true` if the code is in the 3xx (redirection) range.
    ///
    /// # Examples
    ///
    /// ```
    /// StatusCode(302).isRedirect();  // true
    /// StatusCode(200).isRedirect();  // false
    /// ```
    public func isRedirect() -> Bool { self.code >= 300 and self.code < 400 }

    /// Returns `true` if the code is in the 4xx (client error) range.
    ///
    /// # Examples
    ///
    /// ```
    /// StatusCode(404).isClientError();  // true
    /// StatusCode(500).isClientError();  // false
    /// ```
    public func isClientError() -> Bool { self.code >= 400 and self.code < 500 }

    /// Returns `true` if the code is in the 5xx (server error) range.
    ///
    /// # Examples
    ///
    /// ```
    /// StatusCode(503).isServerError();  // true
    /// StatusCode(404).isServerError();  // false
    /// ```
    public func isServerError() -> Bool { self.code >= 500 and self.code < 600 }

    public func clone() -> StatusCode {
        StatusCode(self.code)
    }
}

extend StatusCode {
    /// `200 OK`.
    public static func ok() -> StatusCode { StatusCode(200) }
    /// `201 Created`.
    public static func created() -> StatusCode { StatusCode(201) }
    /// `204 No Content`.
    public static func noContent() -> StatusCode { StatusCode(204) }
    /// `400 Bad Request`.
    public static func badRequest() -> StatusCode { StatusCode(400) }
    /// `401 Unauthorized`.
    public static func unauthorized() -> StatusCode { StatusCode(401) }
    /// `403 Forbidden`.
    public static func forbidden() -> StatusCode { StatusCode(403) }
    /// `404 Not Found`.
    public static func notFound() -> StatusCode { StatusCode(404) }
    /// `405 Method Not Allowed`.
    public static func methodNotAllowed() -> StatusCode { StatusCode(405) }
    /// `409 Conflict`.
    public static func conflict() -> StatusCode { StatusCode(409) }
    /// `422 Unprocessable Entity`.
    public static func unprocessableEntity() -> StatusCode { StatusCode(422) }
    /// `429 Too Many Requests`.
    public static func tooManyRequests() -> StatusCode { StatusCode(429) }
    /// `500 Internal Server Error`.
    public static func internalServerError() -> StatusCode { StatusCode(500) }
}
