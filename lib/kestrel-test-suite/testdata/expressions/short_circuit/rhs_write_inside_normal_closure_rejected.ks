// test: diagnostics
// stdlib: true

// The `and`/`or` right-side thunk is `mutating`, but that must not let a
// normal closure write through its read-only captures. Inside `check`'s
// normal closure, `n` is a read-only view, so the thunk writing `n` is still
// E603, reported against the enclosing normal closure.
module Test

import std.numeric.Int64

func check(f: (Int64) -> Bool) -> Bool { f(3) }

func main() -> lang.i64 {
    var n = 0;
    let r = check({ it == 3 and (if n == 0 { n = 1; true } else { false }) }); // ERROR(E603)
    let _ = r;
    0
}
