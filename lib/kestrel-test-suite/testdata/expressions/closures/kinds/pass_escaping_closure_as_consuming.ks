// test: execution
// stdlib: true
// expect-exit: 0

// Passing table, accepted cell `escaping` -> `consuming`. The conversion makes
// a unique one-shot adapter that owns ONE RETAINED shared handle — a share, not
// a steal — so the original handle is still live afterwards and observes the
// mutation the one-shot call performed.
// See docs/design/closures.md — "Passing: What Fits Where".
module Test

import std.numeric.(Int64)

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }
}

func runOnce(consuming f: consuming () -> Int64) -> Int64 { f() }

@main
func main() -> lang.i64 {
    let e = makeCounter(10);

    if runOnce(e) != 11 { return 1 }
    // Shared environment: the adapter retained it rather than stealing it.
    if e() != 12 { return 2 }

    0
}
