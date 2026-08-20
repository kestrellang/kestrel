// test: execution
// stdlib: true
// expect-exit: 50

// G18 position coverage: the non-binding link of a MULTI-condition
// `while let p = e, cond`. Single-`let` while-let desugars to `loop { match }`
// and has no condition at all, but the multi-condition form falls back to the
// if-break-merge desugaring (`desugar_while_let_chain`) — so the bare `cond`
// link becomes a `HirExpr::If` and reaches `lower_if`.
//
// The `iters` cap keeps a future regression from hanging the suite.

module Test

// `boolValue()` inverts: a NONZERO payload is false.
struct Inverted: BooleanConditional {
    var v: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.v, 0)
    }
}

enum Slot {
    case Some(value: lang.i64)
    case None
}

@main
func main() -> lang.i64 {
    let never = Inverted(v: 3);
    var iters: lang.i64 = 0;
    // The pattern always matches, so only `never.boolValue()` (false) stops
    // the loop. A raw branch on the nonzero payload would loop until the cap.
    while let .Some(x) = Slot.Some(value: 1), never {
        iters = lang.i64_add(iters, 1);
        if lang.i64_signed_gt(iters, 10) {
            return 99;
        }
    }
    lang.i64_add(iters, 50)
}
