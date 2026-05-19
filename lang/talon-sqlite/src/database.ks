/// Database — the public entry point for talon-sqlite.
///
/// ```
/// let db = try Database(":memory:");
/// try db.execute("create table users (id integer primary key, name text)");
///
/// let name = "Alice";
/// try db.execute("insert into users (name) values (\(name))");
///
/// let users = try db.query[User]("select * from users");
/// ```

module talon.sqlite.database

import talon.sqlite.connection.(Connection, execRawOnDb)
import talon.sqlite.executor.(SqliteExecutor)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(FromRow)
import talon.sqlite.error.(SqliteError)
import talon.sqlite.transaction.(Transaction)

/// A SQLite database connection.
///
/// Opens the database file on init, closes it when the value is dropped.
/// Pass `":memory:"` for an in-memory database.
public struct Database: SqliteExecutor {
    var conn: Connection

    /// Opens or creates a SQLite database at the given path.
    ///
    /// Use `":memory:"` for a transient in-memory database.
    public init(path: String) throws SqliteError {
        self.conn = try Connection.open(path);
    }

    public func execute(sql: SQL) -> () throws SqliteError {
        try self.conn.execute(sql);
    }

    public func query[R](sql: SQL) -> Array[R] throws SqliteError where R: FromRow {
        try self.conn.query[R](sql)
    }

    /// Runs `body` inside a BEGIN/COMMIT transaction.
    ///
    /// If `body` throws, the transaction is rolled back and the error
    /// is re-thrown. If `body` returns normally, the transaction commits.
    ///
    /// ```
    /// try db.transaction { tx in
    ///     try tx.execute("insert into users (name) values ('Alice')");
    ///     try tx.execute("insert into users (name) values ('Bob')");
    /// };
    /// ```
    public func transaction(body: (Transaction) -> () throws SqliteError) -> () throws SqliteError {
        try execRawOnDb(self.conn.db, "BEGIN");
        let tx = Transaction(db: self.conn.db);
        match body(tx) {
            .Ok(_) => try execRawOnDb(self.conn.db, "COMMIT"),
            .Err(e) => {
                let _ = execRawOnDb(self.conn.db, "ROLLBACK");
                throw e;
            }
        }
    }
}
