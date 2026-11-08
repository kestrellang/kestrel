/// Errors: `HttpError` (a handler's failure, sent as the response) and
/// `ServerError` (the server itself failed to start or run).

module perch

import http.status.(StatusCode)
import quill.value.(Value)
import quill.json.emitter.(emitJson)
import std.streams.(IoError)

/// A handler failure that becomes the HTTP response.
///
/// Handlers have the shape `(ctx, input) -> R throws HttpError`, so
/// `throw HttpError.notFound()` ends the request with a 404. The body is
/// JSON: `{"error": "<message>"}`.
public struct HttpError: Cloneable, IntoResponse {
    public var status: StatusCode
    public var message: String

    public init(status: StatusCode, message: String) {
        self.status = status;
        self.message = message
    }

    public func clone() -> HttpError = HttpError(self.status.clone(), self.message.clone())

    public consuming func intoResponse() -> Response {
        var body: Dictionary[String, Value] = [:];
        body.insert("error", .Str(self.message.clone()));
        var response = Response.text(self.status.clone(), emitJson(.Obj(body)));
        response.headers.setValue("Content-Type", "application/json");
        response
    }
}

extend HttpError {
    /// 400 Bad Request.
    public static func badRequest(message: String) -> HttpError = HttpError(StatusCode.badRequest(), message)
    public static func badRequest() -> HttpError = HttpError.badRequest("Bad Request")

    /// 401 Unauthorized.
    public static func unauthorized() -> HttpError = HttpError(StatusCode.unauthorized(), "Unauthorized")

    /// 403 Forbidden.
    public static func forbidden() -> HttpError = HttpError(StatusCode.forbidden(), "Forbidden")

    /// 404 Not Found.
    public static func notFound(message: String) -> HttpError = HttpError(StatusCode.notFound(), message)
    public static func notFound() -> HttpError = HttpError.notFound("Not Found")

    /// 409 Conflict.
    public static func conflict(message: String) -> HttpError = HttpError(StatusCode.conflict(), message)

    /// 422 Unprocessable Entity — the request parsed but its content is invalid.
    public static func unprocessable(message: String) -> HttpError =
        HttpError(StatusCode.unprocessableEntity(), message)

    /// 500 Internal Server Error. The message goes to the client, so keep
    /// internals (SQL, file paths) out of it.
    public static func serverError(message: String) -> HttpError =
        HttpError(StatusCode.internalServerError(), message)
    public static func serverError() -> HttpError = HttpError.serverError("Internal Server Error")
}

/// The server failed to start or stopped: bind/accept errors from `listen`,
/// or any startup error converted with `try`.
public struct ServerError: Cloneable {
    public var message: String

    public init(message: String) {
        self.message = message
    }

    public func description() -> String = self.message.clone()

    public func clone() -> ServerError = ServerError(self.message.clone())
}

/// Printable, so `@main func main() -> () throws ServerError` can report it.
extend ServerError: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.message);
    }
}

/// `try` on an `IoError` inside a `throws ServerError` function.
extend Result[T, ServerError]: FromResidual[IoError] {
    public static func fromResidual(consuming residual: IoError) -> Result[T, ServerError] {
        .Err(ServerError(residual.description()))
    }
}

/// An error a handler can propagate with `try`: conform it and `try` in a
/// `throws HttpError` handler converts it, so a library's error type
/// (`SqliteError`, an app's own enum) maps to the status it means.
///
/// ```
/// extend SqliteError: IntoHttpError {
///     public consuming func intoHttpError() -> HttpError = HttpError.serverError()
/// }
/// ```
public protocol IntoHttpError {
    consuming func intoHttpError() -> HttpError
}

/// `try` on an `IntoHttpError` error inside a `throws HttpError` handler.
extend Result[T, HttpError]: FromResidual[E] where E: IntoHttpError {
    public static func fromResidual(consuming residual: E) -> Result[T, HttpError] {
        .Err(residual.intoHttpError())
    }
}
