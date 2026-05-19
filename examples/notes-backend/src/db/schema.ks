module notes.db

import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import talon.sqlite.error.(SqliteError)
import talon.sqlite.connection.(execRawOnDb)

public func initSchema(path: String) -> () throws SqliteError {
    let db = try Database(path);

    // Use execRawOnDb directly for DDL — avoids init(stringLiteral:) codegen bug
    try execRawOnDb(db.conn.db, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, first_name TEXT NOT NULL, last_name TEXT NOT NULL, email TEXT NOT NULL UNIQUE, salt TEXT NOT NULL, password_hash TEXT NOT NULL, created_at TEXT NOT NULL)");

    try execRawOnDb(db.conn.db, "CREATE TABLE IF NOT EXISTS tokens (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER NOT NULL, token TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL)");

    try execRawOnDb(db.conn.db, "CREATE TABLE IF NOT EXISTS folders (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, user_id INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)");

    try execRawOnDb(db.conn.db, "CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, body TEXT NOT NULL, folder_id INTEGER, user_id INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)");
    .Ok(())
}
