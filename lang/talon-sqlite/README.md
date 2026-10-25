# Talon SQLite

SQLite database library for Kestrel. Injection-safe SQL via string interpolation, typed row mapping through the `FromRow` protocol, and RAII connection management. Used by the `notes-backend` example.

## Installation

```toml
[dependencies]
talon-sqlite = { path = "../../lang/talon-sqlite" }
```

**Requires system SQLite.** The package links against the system `sqlite3` library (`[build] link = ["sqlite3"]` in `flock.toml`) — preinstalled on macOS, `libsqlite3-dev` on Debian/Ubuntu.

## Key Types

- **Database** - a connection; opens on init, closes on drop. `SqliteExecutor`.
  - `init(path: String) throws SqliteError` (pass `":memory:"` for in-memory)
  - `execute(sql: SQL) throws SqliteError`
  - `query[R](sql: SQL) -> Array[R] throws SqliteError where R: FromRow`
  - `lastInsertRowId() -> Int64`
  - `transaction(body: (Transaction) -> () throws SqliteError)` - BEGIN/COMMIT, rolls back if `body` throws
- **SharedDatabase** - refcounted, `Cloneable` connection with a shared prepared-statement cache; same API as `Database`. Use when the connection must live inside a `Cloneable` context (e.g. a Perch `AppCtx`).
- **Transaction** - passed to the `transaction` closure; also a `SqliteExecutor`
- **SqliteExecutor** - protocol shared by `Database`/`SharedDatabase`/`Transaction`; accept `some SqliteExecutor` to work with any of them
- **SQL** - parameterized query built by string interpolation (see below)
- **Row** / **FromRow** - `row.read[T](at: index)` typed column access; conform your structs to `FromRow`
- **SqliteValue** - `.Integer(Int64)`, `.Real(Float64)`, `.Text(String)`, `.Null`
- **Bindable** / **FromSqliteValue** - conversion protocols; `Int64`, `Float64`, `String` conform, plus `Optional[T]` for nullable columns on the read side
- **SqliteError** - `.Error(String)`

## Injection-Safe SQL

`SQL` is `ExpressibleByStringInterpolation`: every `\(expr)` becomes a `?` placeholder with the value bound separately, so interpolation cannot inject. Only `Bindable` types may be interpolated — anything else is a compile-time error.

```kestrel
let name = "Alice";
let q: SQL = "select * from users where name = \(name)";
// q.template == "select * from users where name = ?"
// q.bindings == [.Text("Alice")]
```

## Usage

Lifted from the package's own smoke test (`test.ks`):

```kestrel
import talon.sqlite.database.(Database)
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

let db = try Database(":memory:");
try db.execute("create table users (id integer primary key, name text not null)");

let name = "Alice";
try db.execute("insert into users (name) values (\(name))");

let users = try db.query[User]("select id, name from users");
```

Transactions:

```kestrel
try db.transaction { tx in
    try tx.execute("insert into users (name) values ('Alice')");
    try tx.execute("insert into users (name) values ('Bob')");
};
```

Nullable columns read as optionals: `try row.read[String?](at: 2)` maps `NULL` to `.None`, while `try row.read[String](at: 2)` throws on `NULL`.
