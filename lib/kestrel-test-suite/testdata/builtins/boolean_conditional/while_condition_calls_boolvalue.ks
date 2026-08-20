// test: execution
// stdlib: true
// expect-exit: 21

// G18 position coverage: `while`, which desugars to a `HirExpr::If` inside a
// loop and so shares `lower_if`.
//
// The counter is INVERTED (`boolValue() == (remaining == 0)`), so it runs
// exactly one iteration under correct dispatch and zero under a raw branch.
// The `iters` cap is belt-and-braces: a future regression that inverts the
// other way must fail the assertion instead of hanging the suite.

module Test

struct Countdown: BooleanConditional {
    var remaining: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.remaining, 0)
    }

    mutating func bump() {
        self.remaining = lang.i64_add(self.remaining, 1);
    }
}

@main
func main() -> lang.i64 {
    var c = Countdown(remaining: 0);
    var iters: lang.i64 = 0;
    while c {
        iters = lang.i64_add(iters, 1);
        c.bump();
        if lang.i64_signed_gt(iters, 100) {
            return 99;
        }
    }
    // 1 iteration + 20 == 21. A raw branch never enters the loop (20).
    lang.i64_add(iters, 20)
}
