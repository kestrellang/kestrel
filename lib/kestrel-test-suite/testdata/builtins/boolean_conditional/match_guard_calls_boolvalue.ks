// test: execution
// stdlib: true
// expect-exit: 8

// G18 position coverage: a `match` arm guard. This is the ONE condition
// position that is not a `HirExpr::If` — it is consumed directly by
// `DecisionTree::Guard` in mir-lower's pattern lowering, so it needs the
// `boolValue()` coercion wired in separately from `lower_if`.

module Test

// `boolValue()` inverts: a NONZERO payload is false.
struct Inverted: BooleanConditional {
    var v: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.v, 0)
    }
}

enum Choice {
    case First
    case Second
}

@main
func main() -> lang.i64 {
    let never = Inverted(v: 4);
    let c = Choice.First;
    match c {
        .First if never => 2,
        .First => 8,
        .Second => 4
    }
}
