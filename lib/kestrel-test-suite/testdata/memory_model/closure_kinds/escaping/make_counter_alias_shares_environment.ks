// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — the `makeCounter` example, verbatim shape: an
// `escaping` closure owns a heap environment it may mutate, and duplicating
// the handle (`let alias = next;`) RETAINS rather than copies, so the two
// handles are one counter — 11, 12, 13 (reference semantics).
module Test

import std.numeric.Int64

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }   // snapshots count; mutates its own copy
}

@main
func main() -> lang.i64 {
    let next = makeCounter(10);
    if next() != 11 { return 1 }
    let alias = next;                 // retain — shares the environment
    if alias() != 12 { return 2 }
    if next() != 13 { return 3 }      // one counter, two handles
    0
}
