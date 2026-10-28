// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — "Pinned Edge Cases": an `escaping` expected type
// snapshots EACH ITERATION's value when accumulating closures for later calls,
// so the array holds three distinct environments (10/20/30) rather than three
// views of one dead loop binding.
module Test

import std.numeric.Int64

@main
func main() -> lang.i64 {
    var fns = Array[escaping () -> Int64]();
    for i in 1..=3 {
        fns.append({ i * 10 });   // element type supplies the escaping kind
    }
    if (fns(0))() != 10 { return 1 }
    if (fns(1))() != 20 { return 2 }
    if (fns(2))() != 30 { return 3 }
    if (fns(0))() != 10 { return 4 }   // still valid after the loop scope died
    0
}
