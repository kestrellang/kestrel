// test: diagnostics
// stdlib: false
//
// G10: `break outer` from inside a nested loop exits the labeled outer loop,
// so the initializer completes with `a` never stored — E005.
//
// The break-state stack used to push onto `.last_mut()` unconditionally, so
// this break landed in the *inner* frame. The outer frame popped empty, was
// treated as an infinite loop, set `diverged`, and the single `!diverged` gate
// skipped the whole field check. `S()` constructed with `a` uninitialized.

module Main

struct S {
    var a: lang.i64

    init() {
        outer: loop {
            loop {
                break outer;
            }
        }
    } // ERROR: does not initialize all fields
}
