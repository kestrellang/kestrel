// test: execution
// stdlib: true
// expect-exit: 7

// G18 position coverage: `guard <cond> else`, which desugars to a
// `HirExpr::If` and so shares `lower_if`.

module Test

// `boolValue()` inverts: a NONZERO payload is false.
struct Inverted: BooleanConditional {
    var v: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.v, 0)
    }
}

func check(flag: Inverted) -> lang.i64 {
    guard flag else {
        return 7;
    }
    3
}

@main
func main() -> lang.i64 {
    // boolValue() is false -> the guard fails -> 7. A raw branch sees the
    // nonzero payload, lets the guard pass, and returns 3.
    check(Inverted(v: 5))
}
