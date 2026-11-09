// test: execution
// stdlib: true
// expect-exit: 0

// `match xs(i)` reads the element in place instead of cloning it first: only
// the payloads an arm binds are copied. Covers string-literal patterns read
// through the pinned element (they used to leak their test copy), arm bodies
// that grow the array after binding, and a guard that grows it (a match with
// a guard still copies the element, since the guard can move the storage).
module Test

import std.numeric.(Int64)
import std.text.(String)

enum Inner { case S(String) case N(Int64) }
enum Instr { case A(Int64) case T(String, Inner) }

func grow(mutating xs: std.collections.Array[Instr]) -> Bool {
    var k: Int64 = 0;
    while k < 1000 {
        xs.append(Instr.T("filler string that is long enough", Inner.N(k)));
        k = k + 1;
    }
    false
}

@main
func main() -> Int64 {
    var arr: std.collections.Array[Instr] = [
        Instr.T("hello world, a heap string!", Inner.S("second heap string here")),
        Instr.A(7),
    ];

    // Bindings are copied before the body grows the array.
    let r = match arr(0) {
        .T(s, .S(t)) => { let _ = grow(arr); s.bytes.count + t.bytes.count },
        .T(s, _) => s.bytes.count,
        _ => 0,
    };
    if r != 50 { return 1; }
    if arr.count != 1002 { return 2; }

    // String-literal tests through the element.
    let lit = match arr(0) {
        .T("hello world, a heap string!", .S(t)) => t.bytes.count,
        _ => 99,
    };
    if lit != 23 { return 3; }
    let miss = match arr(2) {
        .T("hello world, a heap string!", _) => 1,
        .T("filler string that is long enough", .N(n)) => n,
        _ => 99,
    };
    if miss != 0 { return 4; }

    // A guard that grows the array: the element must still be readable.
    let g = match arr(0) {
        .T(s, _) if grow(arr) => 1,
        .T(s, _) => s.bytes.count,
        _ => 0,
    };
    if g != 27 { return 5; }

    match arr(1) {
        .A(n) => if n != 7 { return 6; },
        _ => return 7,
    }
    0
}
