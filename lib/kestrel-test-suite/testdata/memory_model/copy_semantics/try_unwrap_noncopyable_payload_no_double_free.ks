// test: execution
// stdlib: true

// Regression: `try` on a `Result` carrying a non-Copyable payload double-freed
// the payload. `Result.tryExtract()` is a `consuming func` whose body is
// `match self { .Ok(v) => .Continue(v), ... }`. `Result[T,E]` is a
// *conditionally* Copyable container (`extend Result: Copyable where T,E:
// Copyable`); with unconstrained `T,E` it reports `Bitwise` pre-mono, so
// `match self` lowered to a bitwise *copy* of `self` while keeping the original
// — both then live. Monomorphized with a non-Copyable `T` the copy aliased the
// payload and the surviving original double-freed it when the scrutinee dropped.
// Fix: a mono-dependent @owned match scrutinee is *moved* into the match.
//
// This is the root cause of talon-sqlite's `Database.init` closing its freshly
// opened `sqlite3*` handle (`self.conn = try Connection.open(path)`), which made
// every later `execute`/`query` a SQLITE_MISUSE → heap corruption.
//
// Detector: `Conn` is non-Copyable; its `deinit` writes -1 into a heap cell.
// A correct single-owner move leaves the cell at 42; a double-free / wrong
// alias surfaces as -1.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)

enum OpenErr { case Failed }

struct Conn: not Copyable {
    var cell: Pointer[Int64]
    static func open(value v: Int64) -> Result[Conn, OpenErr] {
        let p = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
        p.write(v);
        .Ok(Conn(cell: p))
    }
    func value() -> Int64 { self.cell.read() }
    deinit { self.cell.write(0 - 1); }
}

// `try`-unwrap a Result whose payload is non-Copyable, then read it.
func openAndRead(value v: Int64) -> Int64 throws OpenErr {
    let c = try Conn.open(value: v);
    let n = c.value();
    .Ok(n)
}

func main() -> lang.i64 {
    match openAndRead(value: 42) {
        .Ok(n) => { if n != 42 { return 1; }; 0 },
        .Err(_) => 2
    }
}
