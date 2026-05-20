module wall.context

import std.memory.(RcBox)
import talon.sqlite.shared_database.(SharedDatabase)

public struct SharedState: Cloneable {
    public var cachedHtml: String
    public var cacheTimestamp: Int64
    public var rateLimits: Dictionary[String, Int64]
    public var blocklist: Set[String]

    public func clone() -> SharedState {
        SharedState(
            cachedHtml: self.cachedHtml.clone(),
            cacheTimestamp: self.cacheTimestamp,
            rateLimits: self.rateLimits.clone(),
            blocklist: self.blocklist.clone()
        )
    }
}

public struct AppCtx: Cloneable {
    public var db: SharedDatabase
    public var state: RcBox[SharedState]

    public func clone() -> AppCtx {
        AppCtx(db: self.db.clone(), state: self.state.clone())
    }
}
