// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — the passing table: escaping -> consuming is ✓, via a
// unique one-shot adapter that owns ONE RETAINED shared handle. The escaping
// value is Cloneable, so the caller's handle survives the consuming call, and
// the adapter's single call mutates the same shared environment.
module Test

import std.numeric.Int64

func runOnce(consuming f: consuming () -> Int64) -> Int64 { f() }

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }
}

@main
func main() -> lang.i64 {
    let next = makeCounter(0);
    if next() != 1 { return 1 }
    if runOnce(next) != 2 { return 2 }   // retained handle, one-shot adapter
    if next() != 3 { return 3 }          // caller's handle still live and shared
    0
}
