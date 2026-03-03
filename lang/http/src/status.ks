// HTTP status codes

module http.status

/// An HTTP status code with its reason phrase.
public struct StatusCode: Cloneable {
    public var code: Int64

    public init(code: Int64) {
        self.code = code
    }

    /// Returns the standard reason phrase for this status code.
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

    public func isSuccess() -> Bool { self.code >= 200 and self.code < 300 }
    public func isRedirect() -> Bool { self.code >= 300 and self.code < 400 }
    public func isClientError() -> Bool { self.code >= 400 and self.code < 500 }
    public func isServerError() -> Bool { self.code >= 500 and self.code < 600 }

    public func clone() -> StatusCode {
        StatusCode(self.code)
    }
}

extend StatusCode {
    public static func ok() -> StatusCode { StatusCode(200) }
    public static func created() -> StatusCode { StatusCode(201) }
    public static func noContent() -> StatusCode { StatusCode(204) }
    public static func badRequest() -> StatusCode { StatusCode(400) }
    public static func unauthorized() -> StatusCode { StatusCode(401) }
    public static func forbidden() -> StatusCode { StatusCode(403) }
    public static func notFound() -> StatusCode { StatusCode(404) }
    public static func methodNotAllowed() -> StatusCode { StatusCode(405) }
    public static func conflict() -> StatusCode { StatusCode(409) }
    public static func unprocessableEntity() -> StatusCode { StatusCode(422) }
    public static func tooManyRequests() -> StatusCode { StatusCode(429) }
    public static func internalServerError() -> StatusCode { StatusCode(500) }
}
