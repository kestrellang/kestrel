// test: diagnostics
// stdlib: false
//
// G12: pins the corrected breakless-loop formula. `loop { doWork(); }` has no
// `break` that exits it, so it diverges and everything after it is dead — no
// E004 on the uninitialized read, only the unreachable-code warning.
//
// The old formula was `body_state.diverged && !contains_break_for`. Here the
// body completes normally, so `diverged` is false and the conjunction answered
// "this loop does not diverge" about an infinite loop. Definite assignment only
// escaped the consequences because a trailing Never-type check re-decided the
// same question; `move_tracking`, which excluded `Loop` from that check, emitted
// a false E500 on exactly this shape. The rule is now `!contains_break_for`
// alone, structurally, in every analyzer.

module Main

func doWork() {}

func test() {
    var x: lang.i64;
    loop {
        doWork();
    }
    let y = x; // WARN: unreachable
}
