/// HTTP response builder with static factories for common status codes.

module perch.response

import http.status.(StatusCode)
import http.headers.(Headers)
import http.cookie.(Cookie)
import http.content.(Content)

/// An HTTP response with status code, headers, and body.
///
/// Use the static factory methods for common responses — `Response.ok(content:)`,
/// `Response.notFound()`, `Response.redirect(to:)`, etc. For custom status
/// codes, use `Response.withStatus(code:, content:)`.
///
/// Responses are value types. The `withHeader` and `withCookie` methods
/// return new copies — the original is unchanged.
///
/// # Examples
///
/// ```
/// Response.ok(Text("Hello, world!"))
/// Response.ok(JsonBody(payload))
/// Response.created(JsonBody(user))
///     .withCookie(Cookie("session", "abc123"))
/// ```
public struct Response: Cloneable {
    public var status: StatusCode
    public var headers: Headers
    public var bodyContent: String

    /// @name Default
    /// Creates a response with the given status, headers, and body.
    public init(status: StatusCode, headers: Headers, bodyContent: String) {
        self.status = status;
        self.headers = headers;
        self.bodyContent = bodyContent
    }

    public func clone() -> Response {
        Response(self.status.clone(), self.headers.clone(), self.bodyContent.clone())
    }
}

// ============================================================================
// CONTENT-BASED FACTORIES
// ============================================================================

extend Response {
    /// Creates a 200 OK response with the given content.
    public static func ok[C](content: C) -> Response where C: Content {
        var hdrs = Headers();
        if let .Some(ct) = content.contentType() {
            hdrs.setValue("Content-Type", ct)
        }
        let bytes = content.toBytes();
        let body = String(fromUtf8: bytes.asSlice()) ?? String();
        Response(StatusCode.ok(), hdrs, body)
    }

    /// Creates a 201 Created response with the given content.
    public static func created[C](content: C) -> Response where C: Content {
        var hdrs = Headers();
        if let .Some(ct) = content.contentType() {
            hdrs.setValue("Content-Type", ct)
        }
        let bytes = content.toBytes();
        let body = String(fromUtf8: bytes.asSlice()) ?? String();
        Response(StatusCode.created(), hdrs, body)
    }

    /// Creates a 400 Bad Request response with the given content.
    public static func badRequest[C](content: C) -> Response where C: Content {
        var hdrs = Headers();
        if let .Some(ct) = content.contentType() {
            hdrs.setValue("Content-Type", ct)
        }
        let bytes = content.toBytes();
        let body = String(fromUtf8: bytes.asSlice()) ?? String();
        Response(StatusCode.badRequest(), hdrs, body)
    }

    /// Creates a 409 Conflict response with the given content.
    public static func conflict[C](content: C) -> Response where C: Content {
        var hdrs = Headers();
        if let .Some(ct) = content.contentType() {
            hdrs.setValue("Content-Type", ct)
        }
        let bytes = content.toBytes();
        let body = String(fromUtf8: bytes.asSlice()) ?? String();
        Response(StatusCode.conflict(), hdrs, body)
    }

    /// Creates a 422 Unprocessable Entity response with the given content.
    public static func unprocessableEntity[C](content: C) -> Response where C: Content {
        var hdrs = Headers();
        if let .Some(ct) = content.contentType() {
            hdrs.setValue("Content-Type", ct)
        }
        let bytes = content.toBytes();
        let body = String(fromUtf8: bytes.asSlice()) ?? String();
        Response(StatusCode.unprocessableEntity(), hdrs, body)
    }

    /// Creates a response with a custom status code and content.
    public static func withStatus[C](code: Int64, content: C) -> Response where C: Content {
        var hdrs = Headers();
        if let .Some(ct) = content.contentType() {
            hdrs.setValue("Content-Type", ct)
        }
        let bytes = content.toBytes();
        let body = String(fromUtf8: bytes.asSlice()) ?? String();
        Response(StatusCode(code), hdrs, body)
    }
}

// ============================================================================
// NO-BODY FACTORIES
// ============================================================================

extend Response {
    /// Creates a 204 No Content response.
    public static func noContent() -> Response {
        Response(StatusCode.noContent(), Headers(), String())
    }

    /// Creates a 401 Unauthorized response.
    public static func unauthorized() -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.unauthorized(), hdrs, "Unauthorized")
    }

    /// Creates a 403 Forbidden response.
    public static func forbidden() -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.forbidden(), hdrs, "Forbidden")
    }

    /// Creates a 404 Not Found response.
    public static func notFound() -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.notFound(), hdrs, "Not Found")
    }

    /// Creates a 405 Method Not Allowed response.
    public static func methodNotAllowed() -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.methodNotAllowed(), hdrs, "Method Not Allowed")
    }

    /// Creates a 429 Too Many Requests response.
    public static func tooManyRequests() -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.tooManyRequests(), hdrs, "Too Many Requests")
    }

    /// Creates a 500 Internal Server Error response.
    public static func internalServerError() -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.internalServerError(), hdrs, "Internal Server Error")
    }

    /// Creates a redirect response (302 Found by default).
    public static func redirect(to url: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Location", url);
        Response(StatusCode(302), hdrs, String())
    }

    /// Creates a redirect response with a specific status code.
    public static func redirect(to url: String, code: Int64) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Location", url);
        Response(StatusCode(code), hdrs, String())
    }
}

// ============================================================================
// COOKIE & HEADER SUPPORT
// ============================================================================

extend Response {
    /// Returns a new response with a `Set-Cookie` header appended.
    public func withCookie(cookie: Cookie) -> Response {
        var hdrs = self.headers;
        hdrs.add("Set-Cookie", cookie.toHeaderValue());
        Response(self.status, hdrs, self.bodyContent)
    }

    /// Returns a new response with an additional header appended.
    public func withHeader(name: String, value: String) -> Response {
        var hdrs = self.headers;
        hdrs.add(name, value);
        Response(self.status, hdrs, self.bodyContent)
    }
}
