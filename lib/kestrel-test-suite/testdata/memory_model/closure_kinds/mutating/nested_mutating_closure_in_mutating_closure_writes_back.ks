// test: execution
// stdlib: true
// expect-exit: 0

// The positive half of nested_mutating_closure_in_normal_closure_rejected.ks:
// when the outer closure is itself `mutating`, its captures are `&mutating`
// views, so a nested `mutating` closure (here, an `and` right side) writes
// all the way back to the original.
module Test

import std.numeric.Int64

func runOnce(mutating action: mutating () -> ()) { action(); }

@main
func main() -> lang.i64 {
    var n = 0;
    runOnce({ runOnce({ n = n + 1; }); });
    if n != 1 { return 1; }
    runOnce({ let _ = true and (if n == 1 { n = 5; true } else { false }); });
    if n != 5 { return 2; }
    0
}
