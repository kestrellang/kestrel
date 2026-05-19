module notes.context

// App-wide context passed to every Perch handler.
// Stores the DB path because Database is not Cloneable — each handler opens its own connection.

public struct AppCtx: Cloneable {
    public var dbPath: String

    public func clone() -> AppCtx {
        AppCtx(dbPath: self.dbPath.clone())
    }
}
