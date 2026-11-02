/// HTTP response value and `IntoResponse`, the protocol a handler's return
/// type conforms to.

module perch

import http.status.(StatusCode)
import http.headers.(Headers)
import http.cookie.(Cookie)
import http.content.(Content, Html, Text, Bytes)

/// An HTTP response with status code, headers, and body.
///
/// Handlers usually return a typed value (`Json[T]`, `Created[R]`,
/// `NoContent`) and let `IntoResponse` build this. Middleware builds one
/// directly to short-circuit: `.Respond(Response.unauthorized())`.
///
/// Responses are value types. The `withHeader` and `withCookie` methods
/// return new copies — the original is unchanged.
public struct Response: Cloneable {
    public var status: StatusCode
    public var headers: Headers
    public var bodyContent: String

    /// Creates a response with the given status, headers, and body.
    public init(status: StatusCode, headers: Headers, bodyContent: String) {
        self.status = status;
        self.headers = headers;
        self.bodyContent = bodyContent
    }

    /// Creates a response with `status` and `content` as the body, setting
    /// `Content-Type` from the content.
    public init[C](status: StatusCode, content: C) where C: Content {
        var headers = Headers();
        if let .Some(contentType) = content.contentType() {
            headers.setValue("Content-Type", contentType)
        }
        let bytes = content.toBytes();
        self.status = status;
        self.headers = headers;
        self.bodyContent = String(fromUtf8: bytes.asSlice()) ?? String()
    }

    public func clone() -> Response {
        Response(self.status.clone(), self.headers.clone(), self.bodyContent.clone())
    }
}

// ============================================================================
// FACTORIES
// ============================================================================

extend Response {
    /// A response with `status` and a plain-text `message` body.
    public static func text(status: StatusCode, message: String) -> Response {
        var headers = Headers();
        headers.setValue("Content-Type", "text/plain; charset=utf-8");
        Response(status, headers, message)
    }

    /// A 401 Unauthorized response.
    public static func unauthorized() -> Response = Response.text(StatusCode.unauthorized(), "Unauthorized")

    /// A 403 Forbidden response.
    public static func forbidden() -> Response = Response.text(StatusCode.forbidden(), "Forbidden")

    /// A 404 Not Found response.
    public static func notFound() -> Response = Response.text(StatusCode.notFound(), "Not Found")

    /// A 400 Bad Request response.
    public static func badRequest() -> Response = Response.text(StatusCode.badRequest(), "Bad Request")

    /// A 500 Internal Server Error response.
    public static func internalServerError() -> Response =
        Response.text(StatusCode.internalServerError(), "Internal Server Error")

    /// A redirect response (302 Found).
    public static func redirect(to url: String) -> Response {
        var headers = Headers();
        headers.setValue("Location", url);
        Response(StatusCode(302), headers, String())
    }

    /// Returns a new response with a `Set-Cookie` header appended.
    public func withCookie(cookie: Cookie) -> Response {
        var headers = self.headers.clone();
        headers.add("Set-Cookie", cookie.toHeaderValue());
        Response(self.status.clone(), headers, self.bodyContent.clone())
    }

    /// Returns a new response with an additional header appended.
    public func withHeader(name: String, value: String) -> Response {
        var headers = self.headers.clone();
        headers.add(name, value);
        Response(self.status.clone(), headers, self.bodyContent.clone())
    }
}

// ============================================================================
// INTO RESPONSE
// ============================================================================

/// A value a handler can return. The status code is part of the type:
/// `Json[T]` is 200, `Created[R]` is 201, `NoContent` is 204.
public protocol IntoResponse {
    consuming func intoResponse() -> Response
}

extend Response: IntoResponse {
    public consuming func intoResponse() -> Response = self
}

/// A plain-text 200 response.
extend String: IntoResponse {
    public consuming func intoResponse() -> Response = Response.text(StatusCode.ok(), self)
}

/// A 201 Created response wrapping another response type's body.
///
/// ```
/// Created(Json(note))
/// ```
public struct Created[R] where R: IntoResponse {
    public var value: R

    public init(value: R) {
        self.value = value
    }
}

extend Created[R]: IntoResponse where R: IntoResponse {
    public consuming func intoResponse() -> Response {
        var response = self.value.intoResponse();
        response.status = StatusCode.created();
        response
    }
}

/// A 204 No Content response.
public struct NoContent: IntoResponse {
    public init() {}

    public consuming func intoResponse() -> Response =
        Response(StatusCode.noContent(), Headers(), String())
}

/// `Html`, `Text`, and `Bytes` bodies from `http.content` are 200 responses
/// with their own `Content-Type`.
extend Html: IntoResponse {
    public consuming func intoResponse() -> Response = Response(StatusCode.ok(), self)
}

extend Text: IntoResponse {
    public consuming func intoResponse() -> Response = Response(StatusCode.ok(), self)
}

extend Bytes: IntoResponse {
    public consuming func intoResponse() -> Response = Response(StatusCode.ok(), self)
}
