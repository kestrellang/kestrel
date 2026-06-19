// test: diagnostics
// stdlib: true

// E504 negative space: the dangle lint claims NOTHING beyond
// `Pointer(to: <same-fn local>)` traced directly (or through a
// single-assignment `let` pointer) into the returned ref. Param-rooted
// captures, stored-pointer heap chains (the Array shape), and `var`
// pointers (reassignable — trace ends) all stay silent. Zero diagnostics
// expected in this file.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

// Param storage is caller-owned — out of the lint's claim.
func fromParam(x: Int64) -> &Int64 {
    Pointer(to: x).value
}

// Stored-pointer chain: validity is the field pointer's contract (the
// Array.at shape) — sound when the pointer targets heap storage.
struct Holder {
    var p: Pointer[Int64]
    func view() -> &Int64 {
        self.p.value
    }
}

// A `var` pointer could be reassigned between init and use; the trace
// deliberately ends rather than guess.
func varPointer() -> &Int64 {
    var x = 1;
    var p = Pointer(to: x);
    p.value
}
