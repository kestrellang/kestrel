// test: execution
// stdlib: true
// expect-exit: 0

// Assigning a branching expression (`if`, `match`) straight into a subscript
// of a struct field. The index is lowered before the right-hand side and used
// after it, so it must survive the branch's block params (it used to be the
// consumed pre-branch value: OSSA "use of consumed value").
module Test

import std.numeric.(Int64)
import std.text.(String)

struct S {
    var a: std.collections.Array[Int64];
    var b: std.collections.Array[Int64];
    var names: std.collections.Array[String];
}

func pick(n: Int64) -> Int64 { n * 2 }

@main
func main() -> Int64 {
    var s = S(a: [1, 2, 3], b: [0, 0], names: ["x", "y"]);
    var c = true;
    let i: Int64 = 2;

    s.b(0) = if c { s.a(i) } else { 0 };
    if s.b(0) != 3 { return 1; }

    c = false;
    s.b(pick(0) + 1) = if c { s.a(i) } else { 7 };
    if s.b(1) != 7 { return 2; }

    s.b(i - 2) = match s.a(0) { 1 => 10, _ => 20 };
    if s.b(0) != 10 { return 3; }

    var arr: std.collections.Array[Int64] = [5, 6];
    arr(pick(0)) = if c { 1 } else { s.a(1) };
    if arr(0) != 2 { return 4; }

    s.names(1) = if c { "no" } else { "yes" };
    if s.names(1) != "yes" { return 5; }
    0
}
