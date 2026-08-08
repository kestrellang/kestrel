// test: execution
// stdlib: true
// expect-exit: 0

// Parse/lowering shape for the design's own signature spelling:
// `func onDone(consuming f: consuming () -> ())` — the parameter ACCESS MODE
// and the closure KIND are the same keyword, adjacent, and must not be
// confused for one another (the first sits before the label, the second before
// the parameter list of the function type).
// See docs/design/closures.md — "consuming: one-shot hand-off".
module Test

import std.numeric.(Int64)

func onDone(consuming f: consuming () -> ()) { f(); }

func onDoneValued(consuming f: consuming () -> Int64) -> Int64 { f() }

@main
func main() -> lang.i64 {
    onDone({ () });

    let v = 5;
    if onDoneValued({ v }) != 5 { return 1 }

    0
}
