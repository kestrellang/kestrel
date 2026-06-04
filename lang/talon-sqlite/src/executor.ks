/// Protocol shared by Database and Transaction.

module talon.sqlite.executor

import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(FromRow)
import talon.sqlite.error.(SqliteError)

/// Common interface for types that can run SQL queries.
///
/// Both `Database` and `Transaction` conform, so functions can accept
/// `some SqliteExecutor` to work with either:
/// ```
/// func insertUser(into db: some SqliteExecutor, name: String) throws SqliteError {
///     try db.execute("insert into users (name) values (\(name))");
/// }
/// ```
public protocol SqliteExecutor {
    func execute(sql: SQL) -> () throws SqliteError
    func query[R](sql: SQL) -> Array[R] throws SqliteError where R: FromRow
    /// rowid of the last successful INSERT on this connection.
    func lastInsertRowId() -> Int64
}
