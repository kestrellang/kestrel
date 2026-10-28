/// Built-in middleware components.

module perch.middleware

import perch.request.(Request)
import perch.context.(MiddlewareResult, Middleware)
import perch.response.(Response)

/// Logging middleware that prints the HTTP method and path for each request.
///
/// Logs one line per request in the format `GET /path`.
///
/// # Examples
///
/// ```
/// var app = App(context: ctx);
/// app.use(Logger[Ctx]());
/// ```
public struct Logger[T]: Middleware[T], Cloneable {
    public func handle(request: Request, ctx: T) -> MiddlewareResult {
         println(request.method.toString() + " " + request.path);
        .Continue(request)
    }

    public func clone() -> Logger[T] = Logger[T]()
}
