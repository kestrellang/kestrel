// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — the passing table: escaping -> normal is ✓. The
// coercion produces a non-owning VIEW of the shared environment (it does not
// re-label the handle), so the callee's calls mutate the same state the caller
// keeps observing afterwards.
module Test

import std.numeric.Int64

func callTwice(f: () -> Int64) -> Int64 { f() + f() }

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }
}

@main
func main() -> lang.i64 {
    let next = makeCounter(0);
    if callTwice(next) != 3 { return 1 }   // 1 + 2 through the normal-kind view
    if next() != 3 { return 2 }            // the caller's handle sees the callee's calls
    0
}
