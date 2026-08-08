// test: execution
// stdlib: true
// expect-exit: 0

// The POSITIVE sibling of view_closure_over_loop_binding_rejected.ks and
// view_closure_over_loop_binding_appended_rejected.ks — the fix-it those two
// point at, and the design's stated answer for accumulating closures
// (docs/design/closures.md, "Behavior Changes from Today" #4: "Returning a
// capturing closure becomes possible — with `escaping` ... in the return
// type").
//
// An `escaping` element type makes each literal build an OWNING environment,
// so every iteration SNAPSHOTS its own `i` at creation rather than viewing the
// iteration binding's storage. Owning kinds freeze nothing — their captures
// are theirs — so the accumulation is legal and the closures still answer
// 10/20/30 long after the loop's storage is gone.
module Test

import std.numeric.Int64
import std.collections.Array

@main
func main() -> lang.i64 {
    var fns = Array[escaping () -> Int64]();
    for i in 1..=3 {
        fns.append({ i * 10 });
    }
    if fns.count != 3 { return 1 }
    if fns(0)() != 10 { return 2 }
    if fns(1)() != 20 { return 3 }
    if fns(2)() != 30 { return 4 }
    0
}
