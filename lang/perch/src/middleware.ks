// Built-in middleware

module perch.middleware

import perch.request.(Request)
import perch.context.(MiddlewareResult)
import perch.response.(Response)

/// Logging middleware that prints the HTTP method and path for each request.
public func logger[T]() -> (Request, T) -> MiddlewareResult {
    { (request: Request, ctx: T) in
        let _ = println(request.method.toString() + " " + request.path);
        .Continue(request)
    }
}
