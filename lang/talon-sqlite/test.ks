/// Quick smoke test for talon-sqlite.

module Test

import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)

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

@main
func main() -> lang.i64 {
    let db = try Database(":memory:");

    try db.execute("create table users (id integer primary key, name text not null)");

    let name1 = "Alice";
    let name2 = "Bob";
    try db.execute("insert into users (name) values (\(name1))");
    try db.execute("insert into users (name) values (\(name2))");

    let users = try db.query[User]("select id, name from users");

    if users.count != 2 { return 1 }
    if users(0).name != "Alice" { return 2 }
    if users(1).name != "Bob" { return 3 }

    0
}
