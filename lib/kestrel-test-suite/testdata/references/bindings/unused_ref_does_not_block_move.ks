// test: diagnostics
// stdlib: true

// A named reference holds its referent only until its last mention: an unused
// `r`, or one already read, does not stop a later move of `x`. Using `r` after
// the move is still E498.
module Test

import std.numeric.(Int64)

struct R: not Copyable { var id: Int64 }
func take(consuming r: R) -> Int64 { r.id }

func unused() -> Int64 {
    let x = R(id: 1);
    let r = &x;
    take(x)
}

func usedBefore() -> Int64 {
    let x = R(id: 1);
    let r = &x;
    let n = r.id;
    take(x) + n
}

func usedAfter() -> Int64 {
    let x = R(id: 1);
    let r = &x;
    let n = take(x); // ERROR(E498)
    n + r.id
}

