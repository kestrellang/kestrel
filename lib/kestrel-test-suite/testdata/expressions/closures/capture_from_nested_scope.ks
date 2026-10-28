// test: diagnostics
// stdlib: false

module Main

// A frame-view closure built inside an `if`/`else` arm keeps its taint through
// the CFG merge (plan D7 / closure-kinds-plan.md "Existing tests the design
// flips": verified 2026-08-07 to pass with ZERO diagnostics before Phase E2a —
// the block param self-rooted and defeated the escape check, so the returned
// closure pointed at a dead stack environment). The diagnostic anchors on the
// `if` expression, which is the merged value's definition site.
func test() -> () -> lang.i64 {
    let outer = 100;
    if true { // ERROR(E494)
        let inner = 10;
        { lang.i64_add(outer, inner) }
    } else {
        { outer }
    }
}
