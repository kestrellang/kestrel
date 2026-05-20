module notes.context

import talon.sqlite.shared_database.(SharedDatabase)

// App-wide context passed to every Perch handler.
// Holds a SharedDatabase — a refcounted connection shared across all handlers.

public struct AppCtx: Cloneable {
    public var db: SharedDatabase

    public func clone() -> AppCtx {
        AppCtx(db: self.db.clone())
    }
}
