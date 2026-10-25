// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md: "view kinds see later writes; owning kinds are
// snapshots." A Copyable capture in a `consuming` closure is bit-copied at
// CREATION time, so mutating the source afterwards is invisible to the call —
// the opposite of the normal-kind behavior pinned by capture_by_value_semantics.
module Test

import std.numeric.Int64

func callOnce(consuming f: consuming () -> Int64) -> Int64 { f() }

@main
func main() -> lang.i64 {
    var x: Int64 = 10;
    // The `let` annotation supplies the kind; the literal is built owning.
    let g: consuming () -> Int64 = { x };
    x = 20;
    if callOnce(g) != 10 { return 1; }
    if x != 20 { return 2; }
    0
}
