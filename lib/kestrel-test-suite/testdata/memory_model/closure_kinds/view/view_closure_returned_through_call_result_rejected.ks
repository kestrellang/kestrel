// test: diagnostics
// stdlib: false

// A view closure that captures a local, passed to a function that wraps its
// closure parameter in a struct and returns it. Returning `Reg(h: { it + k })`
// directly is E494, but routing the same value through `register(f:)` hides
// it: the escape check does not treat a call's result as carrying the views of
// its closure arguments, so `build()` returns a closure viewing its dead local
// `k`. Found in the 2026-10 architecture review at 9767d2dc: the caller reads
// garbage or segfaults (exit 139).
// EXPECTED TO FAIL until a call result carries the provenance of the
// view-carrying arguments it may contain.
module Test

struct Reg {
    var h: (lang.i64) -> lang.i64
}

// Legal on its own: the caller's frame outlives this call.
func register(f: (lang.i64) -> lang.i64) -> Reg { Reg(h: f) }

func build() -> Reg {
    let k: lang.i64 = 5;
    register({ lang.i64_add(it, k) }) // ERROR(E494)
}
