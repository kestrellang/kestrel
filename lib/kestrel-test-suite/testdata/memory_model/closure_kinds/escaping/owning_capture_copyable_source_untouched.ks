// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — owning capture table, Copyable row: the capture is
// a bit-copy and "source afterwards: untouched". The escaping body may mutate
// its OWN copy (E603 is lifted for escaping bodies) without the original ever
// changing, and writes to the original never reach the closure's state.
module Test

import std.numeric.Int64

struct Point {
    var x: Int64
    var y: Int64
}

@main
func main() -> lang.i64 {
    var p = Point(x: 1, y: 2);
    let bump: escaping () -> Int64 = { p.x = p.x + 10; p.x };
    if bump() != 11 { return 1 }
    if p.x != 1 { return 2 }     // the original is untouched — a copy, not a view
    p.x = 100;
    if bump() != 21 { return 3 } // closure state is independent of the original
    if p.x != 100 { return 4 }
    0
}
