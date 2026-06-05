module notes.db

import talon.sqlite.executor.(SqliteExecutor)
import talon.sqlite.sql.(SQL)
import talon.sqlite.error.(SqliteError)
import notes.models.(Note, CountRow)

public func countNotes(db: some SqliteExecutor, userId: Int64) -> Int64 throws SqliteError {
    let rows = try db.query[CountRow]("""
        SELECT COUNT(*) FROM notes WHERE user_id = \(userId)
        """);
    .Ok(rows(0).count)
}

public func listNotes(db: some SqliteExecutor, userId: Int64, limit: Int64, offset: Int64) -> Array[Note] throws SqliteError {
    db.query[Note]("""
        SELECT id, title, body, folder_id, user_id, created_at, updated_at
        FROM notes
        WHERE user_id = \(userId)
        ORDER BY updated_at DESC
        LIMIT \(limit) OFFSET \(offset)
        """)
}

public func findNoteById(db: some SqliteExecutor, id noteId: Int64, userId userId: Int64) -> Note? throws SqliteError {
    let rows = try db.query[Note]("""
        SELECT id, title, body, folder_id, user_id, created_at, updated_at
        FROM notes
        WHERE id = \(noteId) AND user_id = \(userId)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0))) } else { .Ok(.None) }
}

public func createNote(db: some SqliteExecutor, title: String, body: String, folderId: Int64?, userId: Int64, now: String) -> Note throws SqliteError {
    match folderId {
        .Some(fid) => try db.execute("""
            INSERT INTO notes (title, body, folder_id, user_id, created_at, updated_at)
            VALUES (\(title), \(body), \(fid), \(userId), \(now), \(now))
            """),
        .None => try db.execute("""
            INSERT INTO notes (title, body, folder_id, user_id, created_at, updated_at)
            VALUES (\(title), \(body), NULL, \(userId), \(now), \(now))
            """)
    };
    let rows = try db.query[Note]("""
        SELECT id, title, body, folder_id, user_id, created_at, updated_at
        FROM notes
        WHERE rowid = last_insert_rowid()
        """);
    .Ok(rows(0))
}

public func updateNote(db: some SqliteExecutor, id noteId: Int64, title: String, body: String, userId: Int64, now: String) -> Note? throws SqliteError {
    try db.execute("""
        UPDATE notes
        SET title = \(title), body = \(body), updated_at = \(now)
        WHERE id = \(noteId) AND user_id = \(userId)
        """);
    let rows = try db.query[Note]("""
        SELECT id, title, body, folder_id, user_id, created_at, updated_at
        FROM notes
        WHERE id = \(noteId) AND user_id = \(userId)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0))) } else { .Ok(.None) }
}

public func deleteNote(db: some SqliteExecutor, id noteId: Int64, userId userId: Int64) -> () throws SqliteError {
    try db.execute("""
        DELETE FROM notes
        WHERE id = \(noteId) AND user_id = \(userId)
        """);
    .Ok(())
}

public func moveNoteToFolder(db: some SqliteExecutor, id noteId: Int64, folderId: Int64?, userId: Int64, now: String) -> Note? throws SqliteError {
    match folderId {
        .Some(fid) => try db.execute("""
            UPDATE notes
            SET folder_id = \(fid), updated_at = \(now)
            WHERE id = \(noteId) AND user_id = \(userId)
            """),
        .None => try db.execute("""
            UPDATE notes
            SET folder_id = NULL, updated_at = \(now)
            WHERE id = \(noteId) AND user_id = \(userId)
            """)
    };
    let rows = try db.query[Note]("""
        SELECT id, title, body, folder_id, user_id, created_at, updated_at
        FROM notes
        WHERE id = \(noteId) AND user_id = \(userId)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0))) } else { .Ok(.None) }
}
