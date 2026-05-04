/// HTTP response builder with static factories for common status codes.

module perch.response

import http.status.(StatusCode)
import http.headers.(Headers)
import http.cookie.(Cookie)

/// An HTTP response with status code, headers, and body.
///
/// Use the static factory methods for common responses — `Response.ok(text:)`,
/// `Response.notFound()`, `Response.redirect(to:)`, etc. For custom status
/// codes, use `Response.withStatus(code:, message:)`.
///
/// Responses are value types. The `withHeader` and `withCookie` methods
/// return new copies — the original is unchanged.
///
/// # Examples
///
/// ```
/// // Plain text
/// Response.ok(text: "Hello, world!")
///
/// // HTML
/// Response.ok(html: "<h1>Hello</h1>")
///
/// // JSON
/// Response.ok(json: "{\"status\": \"ok\"}")
///
/// // With a cookie
/// let cookie = Cookie("session", "abc123");
/// Response.ok(text: "logged in").withCookie(cookie)
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
// STATIC FACTORIES
// ============================================================================

extend Response {
    /// Creates a 200 OK response with a plain text body.
    public static func ok(text content: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.ok(), hdrs, content)
    }

    /// Creates a 200 OK response with an HTML body.
    public static func ok(html content: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/html; charset=utf-8");
        Response(StatusCode.ok(), hdrs, content)
    }

    /// Creates a 200 OK response with JSON body (pre-serialized string).
    public static func ok(json content: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "application/json");
        Response(StatusCode.ok(), hdrs, content)
    }

    /// Creates a 201 Created response with JSON body.
    public static func created(json content: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "application/json");
        Response(StatusCode.created(), hdrs, content)
    }

    /// Creates a 204 No Content response.
    public static func noContent() -> Response {
        Response(StatusCode.noContent(), Headers(), String())
    }

    /// Creates a 400 Bad Request response with a message.
    public static func badRequest(message: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.badRequest(), hdrs, message)
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

    /// Creates a 409 Conflict response with a message.
    public static func conflict(message: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.conflict(), hdrs, message)
    }

    /// Creates a 422 Unprocessable Entity response with a message.
    public static func unprocessableEntity(message: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode.unprocessableEntity(), hdrs, message)
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
    ///
    /// # Examples
    ///
    /// ```
    /// Response.redirect(to: "/login")
    /// ```
    public static func redirect(to url: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Location", url);
        Response(StatusCode(302), hdrs, String())
    }

    /// Creates a redirect response with a specific status code.
    ///
    /// # Examples
    ///
    /// ```
    /// // 301 Moved Permanently
    /// Response.redirect(to: "/new-path", 301)
    /// ```
    public static func redirect(to url: String, code: Int64) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Location", url);
        Response(StatusCode(code), hdrs, String())
    }

    /// Creates a response with a custom status code and plain text body.
    ///
    /// Use this when none of the named factories fit.
    public static func withStatus(code: Int64, message: String) -> Response {
        var hdrs = Headers();
        hdrs.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(StatusCode(code), hdrs, message)
    }
}

// ============================================================================
// COOKIE SUPPORT
// ============================================================================

extend Response {
    /// Returns a new response with a `Set-Cookie` header appended.
    ///
    /// Multiple cookies are supported — each call appends a separate
    /// `Set-Cookie` header.
    ///
    /// # Examples
    ///
    /// ```
    /// let session = Cookie("session", "abc123");
    /// Response.ok(text: "logged in").withCookie(session)
    /// ```
    public func withCookie(cookie: Cookie) -> Response {
        var hdrs = self.headers;
        hdrs.add("Set-Cookie", cookie.toHeaderValue());
        Response(self.status, hdrs, self.bodyContent)
    }

    /// Returns a new response with an additional header appended.
    ///
    /// Does not replace existing headers with the same name — use this
    /// for headers that can appear multiple times (e.g. `Set-Cookie`,
    /// `Link`).
    ///
    /// # Examples
    ///
    /// ```
    /// Response.ok(text: "hello")
    ///     .withHeader("X-Request-Id", requestId)
    /// ```
    public func withHeader(name: String, value: String) -> Response {
        var hdrs = self.headers;
        hdrs.add(name, value);
        Response(self.status, hdrs, self.bodyContent)
    }
}
