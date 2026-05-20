module notes.db

import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import talon.sqlite.error.(SqliteError)
import notes.models.(AuthToken)

public func lookupToken(db: Database, token: String) -> Int64? throws SqliteError {
    let rows = try db.query[AuthToken]("""
        SELECT token, user_id
        FROM tokens
        WHERE token = \(token)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0).userId)) } else { .Ok(.None) }
}

public func createToken(db: Database, userId: Int64, token: String, now: String) -> () throws SqliteError {
    try db.execute("""
        INSERT INTO tokens (user_id, token, created_at)
        VALUES (\(userId), \(token), \(now))
        """);
    .Ok(())
}
