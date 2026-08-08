// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — "Capture Rules": owning kinds are snapshots.
// An `escaping` expected type makes the literal capture BY OWNERSHIP, so `x`
// is bit-copied at creation; a later `x = 20` is invisible to the closure.
// (The same body in a normal position would be a view and see 20.)
module Test

import std.numeric.Int64

@main
func main() -> lang.i64 {
    var x = 10;
    let g: escaping () -> Int64 = { x };
    x = 20;
    if g() != 10 { return 1 }   // snapshot taken at creation, not a live view
    if x != 20 { return 2 }     // the source variable is untouched by the capture
    if g() != 10 { return 3 }   // the snapshot is stable across calls
    0
}
