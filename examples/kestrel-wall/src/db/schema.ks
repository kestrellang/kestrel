module wall.db

import talon.sqlite.database.(Database)
import talon.sqlite.error.(SqliteError)
import wall.models.(CountRow)

public func initSchema(path: String) -> () throws SqliteError {
    let db = try Database(path);

    try db.execute("""
        CREATE TABLE IF NOT EXISTS blocklist (
            word TEXT PRIMARY KEY
        )
        """);

    try db.execute("""
        CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            message TEXT NOT NULL,
            color TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        """);

    // Seed with welcome notes if the table is empty
    let rows = try db.query[CountRow]("SELECT COUNT(*) FROM notes");
    if rows(0).count == 0 {
        try seedNotes(db)
    };

    .Ok(())
}

func seedNotes(db: Database) -> () throws SqliteError {
    try db.execute("""
        INSERT INTO notes (username, message, color, created_at) VALUES
        ('kestrel-team', 'Welcome to Kestrel Wall! Leave a note for the community.', '#FEFF9C', '2026-01-01T00:00:00Z'),
        ('dino', 'Kestrel: a language that flies.', '#FF7EB3', '2026-01-01T00:01:00Z'),
        ('perch-dev', 'Built with Perch, the Kestrel web framework!', '#7AFCFF', '2026-01-01T00:02:00Z'),
        ('rustacean', 'Types are good, actually.', '#FFA07A', '2026-01-01T00:03:00Z'),
        ('first-timer', 'My first Kestrel program compiled on the first try!', '#98FB98', '2026-01-01T00:04:00Z'),
        ('compiler-nerd', 'Pattern matching makes me happy.', '#DDA0DD', '2026-01-01T00:05:00Z'),
        ('webdev', 'No npm install required.', '#FEFF9C', '2026-01-01T00:06:00Z'),
        ('night-owl', 'Coding at 2am with a new language hits different.', '#FF7EB3', '2026-01-01T00:07:00Z'),
        ('student', 'Learning Kestrel for my PL class. Love the syntax!', '#7AFCFF', '2026-01-01T00:08:00Z'),
        ('open-source', 'Excited to contribute to the ecosystem!', '#FFA07A', '2026-01-01T00:09:00Z'),
        ('minimalist', 'Simple language. Simple framework. Simple joy.', '#98FB98', '2026-01-01T00:10:00Z'),
        ('sql-fan', 'SQL interpolation that compiles to parameterized queries? Amazing.', '#DDA0DD', '2026-01-01T00:11:00Z')
        """);
    .Ok(())
}
