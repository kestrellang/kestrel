/// Middleware control-flow type.
///
/// Every middleware function returns a `MiddlewareResult` to tell the
/// pipeline whether to keep going or to short-circuit with a response.

module perch.context

import perch.request.(Request)
import perch.response.(Response)

/// The result of a middleware function.
///
/// Middleware either continues the pipeline with a (possibly enriched)
/// request, or short-circuits by returning a response immediately. The
/// pipeline evaluates middleware in order: global first, then group-level,
/// then route-level. The first `Respond` wins — no further middleware or
/// handler runs.
///
/// # Examples
///
/// ```
/// // Middleware that requires an Authorization header
/// func requireAuth[T]() -> (Request, T) -> MiddlewareResult {
///     { (request: Request, ctx: T) in
///         match request.header("Authorization") {
///             .Some(_) => .Continue(request),
///             .None => .Respond(Response.unauthorized())
///         }
///     }
/// }
/// ```
public enum MiddlewareResult: Cloneable {
    /// Continue to the next middleware or handler with this request.
    case Continue(Request)

    /// Short-circuit the pipeline and return this response to the client.
    case Respond(Response)

    public func clone() -> MiddlewareResult {
        match self {
            .Continue(req) => .Continue(req.clone()),
            .Respond(res) => .Respond(res.clone())
        }
    }
}

/// A type that can process requests in the middleware pipeline.
///
/// Implement this protocol to create reusable middleware with named
/// types and configurable state.
///
/// # Examples
///
/// ```
/// struct AuthCheck[T]: Middleware[T] {
///     func handle(request: Request, ctx: T) -> MiddlewareResult {
///         match request.header("Authorization") {
///             .Some(_) => .Continue(request),
///             .None => .Respond(Response.unauthorized())
///         }
///     }
///     func clone() -> AuthCheck[T] { AuthCheck[T]() }
/// }
/// ```
public protocol Middleware[T]: Cloneable {
    func handle(request: Request, ctx: T) -> MiddlewareResult
}
