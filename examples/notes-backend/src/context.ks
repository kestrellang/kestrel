module notes.context

import talon.sqlite.shared_database.(SharedDatabase)

// App-wide context passed to every public Perch handler.
// Holds a SharedDatabase — a refcounted connection shared across all handlers.
public struct AppCtx: Cloneable {
    public var db: SharedDatabase

    public func clone() -> AppCtx {
        AppCtx(db: self.db.clone())
    }
}

// Context for routes behind AuthMiddleware: the app context plus the
// authenticated user. Handlers that take this can't run unauthenticated.
public struct AuthCtx: Cloneable {
    public var db: SharedDatabase
    public var userId: Int64

    public init(app ctx: AppCtx, userId id: Int64) {
        self.db = ctx.db.clone();
        self.userId = id;
    }

    public func clone() -> AuthCtx {
        AuthCtx(app: AppCtx(db: self.db.clone()), userId: self.userId)
    }
}
