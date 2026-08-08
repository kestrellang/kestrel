// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — "escaping: a shared, stateful object": escaping
// closures are Cloneable, and `clone()` is a SHALLOW SHARE (a retain), not an
// independent duplicate. An explicitly cloned handle must therefore mutate the
// same captured state as the original.
module Test

import std.numeric.Int64

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }
}

@main
func main() -> lang.i64 {
    let next = makeCounter(0);
    let shared = next.clone();     // share operation, not a deep duplicate
    if next() != 1 { return 1 }
    if shared() != 2 { return 2 }  // continues the original's count
    if next() != 3 { return 3 }
    0
}
