/// Transaction — borrows the database handle for scoped operations.

module talon.sqlite.transaction

import talon.sqlite.connection.(executeOnDb, queryOnDb)
import talon.sqlite.executor.(SqliteExecutor)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(FromRow)
import talon.sqlite.error.(SqliteError)

/// A database transaction that auto-commits or rolls back.
///
/// Created by `Database.transaction` — not constructed directly.
/// Implements `SqliteExecutor` so it can be used interchangeably
/// with `Database`.
public struct Transaction: SqliteExecutor {
    var db: RawPointer

    init(db db: RawPointer) {
        self.db = db;
    }

    public func execute(sql: SQL) -> () throws SqliteError {
        try executeOnDb(self.db, sql);
    }

    public func query[R](sql: SQL) -> Array[R] throws SqliteError where R: FromRow {
        try queryOnDb[R](self.db, sql)
    }
}
