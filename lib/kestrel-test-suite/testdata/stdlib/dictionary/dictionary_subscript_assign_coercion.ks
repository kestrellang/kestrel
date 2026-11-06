// test: execution
// stdlib: true
// expect-exit: 0

// #179: assigning through an Optional-typed subscript setter must coerce the
// RHS to the setter's `newValue` type. `b("a") = 3` should be accepted (the
// RHS `Int64` promotes to `Optional[Int64]`), not rejected with a type
// mismatch. (Dictionary's own `d(key)` no longer has a setter, so the
// subscript lives on a wrapper here.)
module Test

import std.numeric.Int64
import std.collections.(Dictionary)

struct Bag {
    var d: Dictionary[String, Int64]

    subscript(key: String) -> Int64? {
        get { self.d(key) }
        set {
            match newValue {
                .Some(v) => { self.d.insert(key, v); },
                .None => { self.d.remove(key); },
            }
        }
    }
}

@main
func main() -> lang.i64 {
    var d = Bag(d: [:]);
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
