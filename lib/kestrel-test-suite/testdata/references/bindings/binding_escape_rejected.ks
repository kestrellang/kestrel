// test: diagnostics
// stdlib: false

// Named-binding lifetime rules: returning a binding rooted at a local is
// the stage-1 escape error (E494); a binding still used after an
// `if`/`match`/loop boundary is the binding E497 — bindings never cross
// blocks (no @guaranteed block params in this version). Both errors point
// at the binding's `let`.
module Test

func dangleBinding() -> &lang.i64 {
    var x: lang.i64 = 5;
    let r = &x;
    return r // ERROR(E494)
}

func useAfterMerge(c: lang.i1) -> lang.i64 {
    var x: lang.i64 = 1;
    let r = &x; // ERROR(E497)
    if c {
        x = 2;
    }
    r
}
