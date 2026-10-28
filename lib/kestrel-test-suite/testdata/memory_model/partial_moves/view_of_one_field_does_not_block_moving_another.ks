// test: execution
// stdlib: true
// expect-exit: 0

// Fuzz M8: a view closure capturing `p.a.v` freezes only that place, so
// consuming the disjoint field `p.b` while the closure is live is fine.
module Test

import std.numeric.(Int64)

struct R: not Copyable { var id: Int64 }
struct V { var v: Int64 }
struct P: not Copyable { var a: V; var b: R }

func take(consuming r: R) -> Int64 { r.id }
func call(f: () -> Int64) -> Int64 { f() }

@main
func main() -> Int64 {
    let p = P(a: V(v: 1), b: R(id: 2));
    let f = { p.a.v };
    let n = take(p.b);
    n + call(f) - 3
}
