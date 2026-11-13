module notes.middleware

import perch.(Request, Response, Middleware, MiddlewareResult)
import notes.context.(AppCtx, AuthCtx)
import notes.db.(lookupToken)

// Resolves `Authorization: Bearer <token>` to a user and upgrades the
// group's context from AppCtx to AuthCtx. Stored once by Perch and called
// by borrow, so it needs no Cloneable.
public struct AuthMiddleware: Middleware[AppCtx] {
    type Output = AuthCtx;

    public func handle(request: Request, ctx: AppCtx) -> MiddlewareResult[AuthCtx] {
        guard let some header = request.header("Authorization") else {
            return .Respond(Response.unauthorized())
        }
        guard header.starts(with: "Bearer ") else {
            return .Respond(Response.unauthorized())
        }
        let token = header.asSlice().subslice(from: 7, to: header.bytes.count).toOwned();
        guard let .Ok(some userId) = lookupToken(ctx.db, token) else {
            return .Respond(Response.unauthorized())
        }
        .Continue(AuthCtx(app: ctx, userId: userId))
    }
}
