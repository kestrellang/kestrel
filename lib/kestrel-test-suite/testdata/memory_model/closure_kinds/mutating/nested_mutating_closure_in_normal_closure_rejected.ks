// test: diagnostics
// stdlib: true

// A `mutating` closure nested inside a normal closure writes through the
// normal closure's captures, which are read-only views. The write is E603 on
// the outer normal closure, the same as writing the capture directly.
module Test

import std.numeric.Int64

func check(f: () -> Bool) -> Bool { f() }
func runOnce(mutating action: mutating () -> ()) { action(); }

func main() -> lang.i64 {
    var n = 0;
    let r = check({ runOnce({ n = 1; }); true }); // ERROR(E603)
    let _ = r;
    0
}
