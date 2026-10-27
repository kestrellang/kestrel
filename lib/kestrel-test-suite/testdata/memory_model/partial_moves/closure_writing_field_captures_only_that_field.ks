// test: execution
// stdlib: true
// expect-exit: 0

// A view closure that writes a field captures just that field by address
// (partial-moves.md rule 8): it may run alongside a `mutating` borrow of a
// disjoint field, and the local's other fields stay movable.
module Test

import std.numeric.(Int64)

struct R: not Copyable { var id: Int64 }
struct P { var a: Int64; var b: Int64 }
struct Q: not Copyable { var r: R; var n: Int64 }

func w(mutating a: Int64, mutating f: mutating () -> ()) { a = 7; f(); }
func run(mutating f: mutating () -> ()) { f() }
func take(consuming r: R) -> Int64 { r.id }

@main
func main() -> Int64 {
    var p = P(a: 1, b: 2);
    w(p.a, { p.b = 3 });
    if p.a != 7 { return 1 }
    if p.b != 3 { return 2 }
    var q = Q(r: R(id: 5), n: 0);
    run({ q.n = q.n + 1 });
    run({ q.n = q.n + 1 });
    if q.n != 2 { return 3 }
    if take(q.r) != 5 { return 4 }
    0
}
