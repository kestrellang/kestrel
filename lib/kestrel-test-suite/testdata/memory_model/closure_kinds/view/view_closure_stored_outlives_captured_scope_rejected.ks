// test: diagnostics
// stdlib: false

// Plan D8 (docs/plans/closure-kinds/closure-kinds-plan.md), the SCOPE-DEPTH
// rule: freezing move sites alone is not enough. `FreezeInfo` records the
// captured place's SCOPE DEPTH, and every propagation point (Let / Assign /
// aggregate-store / call-arg / Return) rejects storing a view-carrying value
// into a binding whose scope is SHALLOWER than the captured place's; block
// exit restores the frozen set only after that check.
//
// This is the review's use-after-free counterexample. `r` is declared inside
// the inner block and dies at its exit, but `g` — declared in the enclosing
// scope — is assigned a closure that views `r`. No move and no `deinit` ever
// happens, so a move-site-only freeze sees nothing wrong and `g()` below reads
// freed storage with no diagnostic at all.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

func test() -> lang.i64 {
    var g: () -> lang.i64 = { () in 0 };   // capture-free literal: carries no view
    if lang.i64_eq(1, 1) {
        let r = Res(v: 9);
        g = { r.v };                       // ERROR(E507)
    }                                      // `r` dies here; `g` would dangle
    g()
}
