module notes.db

import talon.sqlite.executor.(SqliteExecutor)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)
import notes.models.(User)

public struct PasswordRow: FromRow, Cloneable {
    public var id: Int64
    public var salt: String
    public var passwordHash: String

    public static func fromRow(row: Row) -> PasswordRow throws SqliteError {
        PasswordRow(
            id: try row.read[Int64](at: 0),
            salt: try row.read[String](at: 1),
            passwordHash: try row.read[String](at: 2)
        )
    }

    public func clone() -> PasswordRow {
        PasswordRow(
            id: self.id,
            salt: self.salt.clone(),
            passwordHash: self.passwordHash.clone()
        )
    }
}

public func findUserByEmail(db: some SqliteExecutor, email: String) -> User? throws SqliteError {
    let rows = try db.query[User]("""
        SELECT id, first_name, last_name, email, created_at
        FROM users
        WHERE email = \(email)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0))) } else { .Ok(.None) }
}

public func findPasswordByEmail(db: some SqliteExecutor, email: String) -> PasswordRow? throws SqliteError {
    let rows = try db.query[PasswordRow]("""
        SELECT id, salt, password_hash
        FROM users
        WHERE email = \(email)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0))) } else { .Ok(.None) }
}

public func createUser(db: some SqliteExecutor, firstName: String, lastName: String, email: String, salt: String, passwordHash: String, now: String) -> User throws SqliteError {
    try db.execute("""
        INSERT INTO users (first_name, last_name, email, salt, password_hash, created_at)
        VALUES (\(firstName), \(lastName), \(email), \(salt), \(passwordHash), \(now))
        """);
    let rows = try db.query[User]("""
        SELECT id, first_name, last_name, email, created_at
        FROM users
        WHERE rowid = last_insert_rowid()
        """);
    .Ok(rows(0))
}
