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
