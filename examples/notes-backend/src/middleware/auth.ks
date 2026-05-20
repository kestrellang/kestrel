module notes.middleware

import perch.request.(Request)
import perch.response.(Response)
import perch.context.(MiddlewareResult, Middleware)
import talon.sqlite.database.(Database)
import notes.context.(AppCtx)
import notes.db.(lookupToken)

public struct AuthMiddleware: Middleware[AppCtx], Cloneable {
    public func handle(request: Request, ctx: AppCtx) -> MiddlewareResult {
        guard let .Some(authHeader) = request.header("Authorization") else {
            return .Respond(Response.unauthorized())
        }
        guard authHeader.starts(with: "Bearer ") else {
            return .Respond(Response.unauthorized())
        }

        let token = authHeader.asSlice().subslice(from: 7, to: authHeader.byteCount).toOwned();

        guard let .Ok(db) = Database(ctx.dbPath) else {
            return .Respond(Response.internalServerError())
        }
        guard let .Ok(some userId) = lookupToken(db, token) else {
            return .Respond(Response.unauthorized())
        }

        var req = request;
        req.store.insert("userId", "\(userId)");
        .Continue(req)
    }

    public func clone() -> AuthMiddleware { AuthMiddleware() }
}
