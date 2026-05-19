module notes.middleware

import perch.request.(Request)
import perch.response.(Response)
import perch.context.(MiddlewareResult, Middleware)
import talon.sqlite.database.(Database)
import notes.context.(AppCtx)
import notes.db.(lookupToken)

public struct AuthMiddleware: Middleware[AppCtx], Cloneable {
    public func handle(request: Request, ctx: AppCtx) -> MiddlewareResult {
        let authHeader = match request.header("Authorization") {
            .Some(h) => h,
            .None => return .Respond(Response.unauthorized())
        };

        if not authHeader.starts(with: "Bearer ") {
            return .Respond(Response.unauthorized())
        };

        let token = authHeader.asSlice().subslice(from: 7, to: authHeader.byteCount).toOwned();

        let db = match Database(ctx.dbPath) {
            .Ok(db) => db,
            .Err(_) => return .Respond(Response.internalServerError())
        };

        let userId = match lookupToken(db, token) {
            .Ok(.Some(id)) => id,
            _ => return .Respond(Response.unauthorized())
        };

        var req = request;
        req.store.insert("userId", userId.formatted());
        .Continue(req)
    }

    public func clone() -> AuthMiddleware { AuthMiddleware() }
}

// Hashes a password with a salt using DefaultHasher.
public func hashPassword(password: String, salt: String) -> String {
    var hasher = DefaultHasher();
    salt.hash(into: hasher);
    password.hash(into: hasher);
    hasher.finish().formatted()
}

// Generates a simple salt from the email (deterministic but unique per user).
public func generateSalt(email: String) -> String {
    var hasher = DefaultHasher();
    email.hash(into: hasher);
    "salt-" + hasher.finish().formatted()
}

// Generates a token string from userId and salt.
public func generateToken(userId: Int64, salt: String) -> String {
    var hasher = DefaultHasher();
    userId.formatted().hash(into: hasher);
    salt.hash(into: hasher);
    "tok-" + hasher.finish().formatted()
}
