// Middleware result type

module perch.context

import perch.request.(Request)
import perch.response.(Response)

/// The result of a middleware function.
///
/// Middleware either continues the pipeline with a (possibly enriched) request,
/// or short-circuits by returning a response immediately.
public enum MiddlewareResult {
    /// Continue to the next middleware/handler with this request.
    case Continue(Request)

    /// Stop the pipeline and return this response.
    case Respond(Response)
}
