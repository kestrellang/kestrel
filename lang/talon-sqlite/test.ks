/// Quick smoke test for talon-sqlite.

module Test

import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)
import uuid.(UUID)

struct User: FromRow {
    var id: Int64
    var name: String

    static func fromRow(row: Row) -> User throws SqliteError {
        User(
            id: try row.read[Int64](at: 0),
            name: try row.read[String](at: 1)
        )
    }
}

struct Todo: FromRow {
    var id: UUID
    var task: String

    static func fromRow(row: Row) -> Todo throws SqliteError {
        Todo(
            id: try row.read[UUID](at: 0),
            task: try row.read[String](at: 1)
        )
    }
}

// The checks throw on a database error, so they run in `run`, and `main`
// turns an error into its own exit code.
@main
func main() -> Int64 {
    match run() {
        .Ok(code) => code,
        .Err(_) => 100
    }
}

func run() -> Int64 throws SqliteError {
    let db = try Database(":memory:");

    try db.execute("create table users (id integer primary key, name text not null)");

    let name1 = "Alice";
    let name2 = "Bob";
    try db.execute("insert into users (name) values (\(name1))");
    try db.execute("insert into users (name) values (\(name2))");

    let users = try db.query[User]("select id, name from users");

    if users.count != 2 { return .Ok(1) }
    if users(0).name != "Alice" { return .Ok(2) }
    if users(1).name != "Bob" { return .Ok(3) }

    // UUIDs bind and read back as TEXT.
    try db.execute("create table todos (id text primary key, task text not null)");
    let id = UUID.v4();
    let task = "write tests";
    try db.execute("insert into todos values (\(id), \(task))");
    let todos = try db.query[Todo]("select id, task from todos where id = \(id)");
    if todos.count != 1 { return .Ok(4) }
    if todos(0).id != id { return .Ok(5) }
    let stored = try db.query[User]("select 1, id from todos");
    if stored(0).name != "\(id)" { return .Ok(6) }

    // Text that isn't a UUID is a read error, not a nil UUID.
    let bad = "not-a-uuid";
    try db.execute("insert into todos values (\(bad), \(task))");
    if db.query[Todo]("select id, task from todos where id = \(bad)").isOk { return .Ok(7) }

    .Ok(0)
}
