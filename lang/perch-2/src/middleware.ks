/// Typed middleware: each middleware takes the group's context and
/// produces the context the next layer (or the handler) sees.

module perch

/// What a middleware decided: continue with the next context, or answer
/// the request now.
public enum MiddlewareResult[Ctx] {
    /// Continue to the next middleware or handler with this context.
    case Continue(Ctx)

    /// Short-circuit and send this response; no handler runs.
    case Respond(Response)
}

/// A request-processing layer that turns a `Ctx` into an `Output` context.
///
/// `group.using(m)` returns a group whose context is `m.Output`, so a
/// handler that takes `AuthCtx` can only be registered behind the
/// middleware that produces one. Perch stores the middleware once and
/// calls it by borrow; it need not be `Cloneable`.
///
/// ```
/// struct Auth: Middleware[AppCtx] {
///     type Output = AuthCtx;
///     func handle(request: Request, ctx: AppCtx) -> MiddlewareResult[AuthCtx] {
///         guard let some user = lookup(request) else {
///             return .Respond(Response.unauthorized())
///         }
///         .Continue(AuthCtx(app: ctx, user: user))
///     }
/// }
/// ```
public protocol Middleware[Ctx] {
    type Output
    func handle(request: Request, ctx: Ctx) -> MiddlewareResult[Output]
}

/// Prints `METHOD /path` for every request and passes the context through.
public struct Logger[Ctx]: Middleware[Ctx] {
    type Output = Ctx

    public init() {}

    public func handle(request: Request, ctx: Ctx) -> MiddlewareResult[Ctx] {
        println(request.method.toString() + " " + request.path);
        .Continue(ctx)
    }
}
