// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Stores through a subscript into a nested stored field or tuple element
// (`arr(0).i.x = v`, `arr(0).0 = v`, `+=`) get the element, update it, and
// set it back. They used to update a temporary copy and be lost.

module Test

struct In { var x: Int64; }
struct S { var i: In; var h: Int64; var t: (In, Int64); }

@main
func main() -> lang.i64 {
    var arr: [S] = [S(i: In(x: 1), h: 3, t: (In(x: 5), 6))];
    arr(0).i.x = 10;
    if arr(0).i.x != 10 { return 1 }
    arr(0).h = 77;
    if arr(0).h != 77 { return 2 }
    arr(0).i.x += 5;
    if arr(0).i.x != 15 { return 3 }
    arr(0).t.0.x = 20;
    if arr(0).t.0.x != 20 { return 4 }

    var tarr: [(Int64, Int64)] = [(1, 2)];
    tarr(0).1 = 44;
    if tarr(0).1 != 44 { return 5 }

    var nested: [[(Int64, Int64)]] = [[(1, 2)]];
    nested(0)(0).0 = 9;
    if nested(0)(0).0 != 9 { return 6 }
    0
}
