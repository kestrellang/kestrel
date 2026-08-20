// test: execution
// stdlib: true
// expect-exit: 0

// The boxing path's MINIMAL shape (fragility audit F4): an escaping closure
// whose only capture is a bare primitive — no wrapping struct, no heap payload,
// no `deinit` — created in a frame that then returns.
//
// The synthesized environment is a single `Int64` word, so the shared box's
// `init(consuming value: E)` is the only thing standing between the captured
// integer and the closure value's handle word. Picking the WRONG one-parameter
// initializer on the box (`RcBox` also has a private
// `init(inner: Pointer[RcBoxStorage[T]])` that adopts an already-allocated
// block) stores `1234` straight into the handle field, and the first call
// dereferences it as a pointer.
//
// `escaping_snapshot_at_creation.ks` captures a bare `Int64` too, but creates
// AND calls in the same frame; the neighbouring cross-frame tests all capture
// something larger. This is the smallest cross-frame round trip.
module Test

import std.numeric.Int64

func capture(value: Int64) -> escaping () -> Int64 {
    { () in value }
}

@main
func main() -> lang.i64 {
    let f = capture(1234);
    if f() != 1234 { return 1 }   // the word survived the defining frame
    if f() != 1234 { return 2 }   // ...and the environment is still there after a call

    let g = capture(-7);          // a second, independent box
    if g() != -7 { return 3 }
    if f() != 1234 { return 4 }   // the two environments did not alias
    0
}
