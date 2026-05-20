module wall.db

import talon.sqlite.executor.(SqliteExecutor)
import talon.sqlite.error.(SqliteError)
import wall.models.(WallNote, CountRow, BlocklistWord)

public func getRecentNotes(db: some SqliteExecutor, limit: Int64) -> Array[WallNote] throws SqliteError {
    db.query[WallNote]("""
        SELECT id, username, message, color, created_at
        FROM notes
        ORDER BY id DESC
        LIMIT \(limit)
        """)
}

public func createNote(db: some SqliteExecutor, username: String, message: String, color: String, now: String) -> WallNote throws SqliteError {
    try db.execute("""
        INSERT INTO notes (username, message, color, created_at)
        VALUES (\(username), \(message), \(color), \(now))
        """);
    let rows = try db.query[WallNote]("""
        SELECT id, username, message, color, created_at
        FROM notes
        WHERE rowid = last_insert_rowid()
        """);
    .Ok(rows(0))
}

public func countNotes(db: some SqliteExecutor) -> Int64 throws SqliteError {
    let rows = try db.query[CountRow]("SELECT COUNT(*) FROM notes");
    .Ok(rows(0).count)
}

public func loadBlocklist(db: some SqliteExecutor) -> Set[String] throws SqliteError {
    let rows = try db.query[BlocklistWord]("SELECT word FROM blocklist");
    var result = Set[String]();
    var i: Int64 = 0;
    while i < rows.count {
        result.insert(rows(unchecked: i).word);
        i = i + 1
    };
    .Ok(result)
}
