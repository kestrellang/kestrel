// test: diagnostics
// stdlib: false
//
// G12 regression: the false E500. `loop { doWork(); }` has no `break` that
// exits it, so `consume(h)` after it never runs. Move tracking used the formula
// `body_state.diverged && !contains_break_for` — the body completes normally,
// so `diverged` was false and the loop was called non-diverging. Every other
// analyzer's copy of that formula was accidentally rescued by a trailing
// Never-type check, but move tracking explicitly excluded `Loop` from it, so
// this shape reported "use of moved value" on a line `dead_code` simultaneously
// called unreachable. Expect ONLY the unreachable warning here.
//
// The sibling `move_in_infinite_loop_is_definitely_moved.ks` covers the
// break-containing loop, where post-loop code IS reachable and E500 is correct.

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func consume(consuming h: Handle) {}

func doWork() {}

func test() {
    var h = Handle(fd: 42);
    consume(h);
    loop {
        doWork();
    }
    consume(h) // WARN: unreachable
}
