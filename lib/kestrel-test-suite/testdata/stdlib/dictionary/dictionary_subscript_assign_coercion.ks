// test: execution
// stdlib: true
// expect-exit: 0

// #179: assigning through a Dictionary subscript must coerce the RHS to the
// setter's Optional `newValue` type. `d("a") = 3` should be accepted (the RHS
// `Int64` promotes to `Optional[Int64]`), not rejected with a type mismatch.
module Test

import std.numeric.Int64

@main
func main() -> lang.i64 {
    var d: [String: Int64] = [:];
    d("a") = 3;          // bare value, coerces to Optional[Int64]
    d("b") = 7;
    if d("a")! != 3 { return 1; }
    if d("b")! != 7 { return 2; }

    // Explicit .Some still works (no regression).
    d("a") = .Some(5);
    if d("a")! != 5 { return 3; }

    // Assigning null removes / leaves absent.
    d("b") = null;
    if d("b").isSome() { return 4; }

    0
}
