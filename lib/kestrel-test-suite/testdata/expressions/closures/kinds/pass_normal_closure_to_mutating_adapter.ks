// test: execution
// stdlib: true
// expect-exit: 0

// Passing table, accepted cell normal -> `mutating`. The conversion builds an
// exclusive-call ADAPTER over the same frame views: it does not recompile the
// body or grant it new capture operations, and the adapter (not the original
// `let` binding) supplies the mutable place — so a `let`-bound normal closure
// passes fine. Because the views are live, later writes to `base` are visible.
// See docs/design/closures.md — "Passing: What Fits Where".
module Test

import std.numeric.(Int64)

func applyTwice(mutating f: mutating () -> Int64) -> Int64 { f() + f() }

@main
func main() -> lang.i64 {
    var base = 5;
    let n: () -> Int64 = { base };

    if applyTwice(n) != 10 { return 1 }

    base = 6;
    if applyTwice(n) != 12 { return 2 }

    0
}
