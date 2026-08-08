// test: diagnostics
// stdlib: true

// The ACCUMULATION spelling of the same dangle as
// view_closure_over_loop_binding_rejected.ks: nothing is assigned to an outer
// binding, the view is handed to a container that lives longer instead.
//
// A call exposes exactly one lexical destination the caller can name — a place
// the callee may WRITE — so `fns` (the `mutating` receiver of `append`) is the
// destination here, and it outlives the iteration binding `i` the appended
// closure views. Calling the collected closures after the loop would read dead
// storage.
//
// Note what this must NOT reject: `iter.forEach { local }` hands a closure to
// a parameter whose signature SPELLS a function type, i.e. the callee CALLS
// it rather than storing it, and the callee frame is strictly deeper. Only a
// slot typed by something else (here `Array[T].append(consuming element: T)`,
// a generic `T`) is a container the value comes to rest in.
module Test

import std.numeric.Int64
import std.collections.Array

func test() -> Int64 {
    var fns = Array[() -> Int64]();
    for i in 1..=3 {
        fns.append({ i * 10 });   // ERROR(E507)
    }
    fns.count
}
