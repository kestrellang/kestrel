// test: execution
// stdlib: true
// expect-exit: 0

// Regression: multiple `Array[T]` subscripts with different `T`s in the same
// body must each project to their own element type, even though all share
// the same `Int64` index type.
//
// `Array.subscript[I](unchecked index: I) -> I.ArrayYield where I: ArrayIndex[T]`
// allocates a fresh `I` TyVar at every call site. Each fresh I is later
// unified with the body's `i: Int64`, so all of them canonicalize to the
// same Int64 slot. The witness-args cache used to canonicalize the container
// at lookup time, then scan for "any tv that resolves to canonical" — which
// silently picked whichever entry HashMap iteration returned first. As a
// result, every subscript yielded the same element type — whichever call
// site's record came up first in iteration order.
//
// After fix: solve_associated keys the witness lookup on the *exact*
// container TyVar from the constraint, so each call site sees its own
// recorded `[T]` args.
//
// Three different element types are used so the test catches the bug
// regardless of HashMap iteration order: under the buggy code all three
// sites return the same type, so at least two of the explicit annotations
// must mismatch and the program fails to compile.

module assoc_subscript_regression

import std.text.String
import std.numeric.Int64
import std.numeric.Float64
import std.collections.Array

func main() -> lang.i64 {
    var ints = Array[Int64]();
    ints.append(7);

    var floats = Array[Float64]();
    floats.append(2.5);

    var strs = Array[String]();
    strs.append("hi");

    let i: Int64 = 0;
    let n: Int64 = ints(unchecked: i);
    let f: Float64 = floats(unchecked: i);
    let s: String = strs(unchecked: i);

    if n != 7 { return 1 }
    if f != 2.5 { return 2 }
    if s.byteCount != 2 { return 3 }
    0
}
