module notes.db

import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import talon.sqlite.error.(SqliteError)
import notes.models.(Folder, CountRow)

public func countFolders(db: Database, userId: Int64) -> Int64 throws SqliteError {
    let rows = try db.query[CountRow]("""
        SELECT COUNT(*) FROM folders WHERE user_id = \(userId)
        """);
    .Ok(rows(0).count)
}

public func listFolders(db: Database, userId: Int64, limit: Int64, offset: Int64) -> Array[Folder] throws SqliteError {
    db.query[Folder]("""
        SELECT id, name, user_id, created_at, updated_at
        FROM folders
        WHERE user_id = \(userId)
        ORDER BY created_at DESC
        LIMIT \(limit) OFFSET \(offset)
        """)
}

public func findFolderById(db: Database, id folderId: Int64, userId userId: Int64) -> Folder? throws SqliteError {
    let rows = try db.query[Folder]("""
        SELECT id, name, user_id, created_at, updated_at
        FROM folders
        WHERE id = \(folderId) AND user_id = \(userId)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0))) } else { .Ok(.None) }
}

public func createFolder(db: Database, name: String, userId: Int64, now: String) -> Folder throws SqliteError {
    try db.execute("""
        INSERT INTO folders (name, user_id, created_at, updated_at)
        VALUES (\(name), \(userId), \(now), \(now))
        """);
    let rows = try db.query[Folder]("""
        SELECT id, name, user_id, created_at, updated_at
        FROM folders
        WHERE rowid = last_insert_rowid()
        """);
    .Ok(rows(0))
}

public func updateFolder(db: Database, id folderId: Int64, name: String, userId: Int64, now: String) -> Folder? throws SqliteError {
    try db.execute("""
        UPDATE folders
        SET name = \(name), updated_at = \(now)
        WHERE id = \(folderId) AND user_id = \(userId)
        """);
    let rows = try db.query[Folder]("""
        SELECT id, name, user_id, created_at, updated_at
        FROM folders
        WHERE id = \(folderId) AND user_id = \(userId)
        """);
    if rows.count > 0 { .Ok(.Some(rows(0))) } else { .Ok(.None) }
}

public func deleteFolder(db: Database, id folderId: Int64, userId userId: Int64) -> () throws SqliteError {
    try db.execute("""
        UPDATE notes SET folder_id = NULL
        WHERE folder_id = \(folderId) AND user_id = \(userId)
        """);
    try db.execute("""
        DELETE FROM folders
        WHERE id = \(folderId) AND user_id = \(userId)
        """);
    .Ok(())
}
