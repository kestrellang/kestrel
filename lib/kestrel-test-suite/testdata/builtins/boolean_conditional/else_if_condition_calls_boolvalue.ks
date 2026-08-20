// test: execution
// stdlib: true
// expect-exit: 5

// G18 position coverage: the `else if` link. `else if` is a nested
// `HirExpr::If` in the else block, so it goes through `lower_if` like the
// leading `if` — this pins that it really does.

module Test

// `boolValue()` inverts: a NONZERO payload is false.
struct Inverted: BooleanConditional {
    var v: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.v, 0)
    }
}

@main
func main() -> lang.i64 {
    let never = Inverted(v: 3);   // boolValue() == false
    let always = Inverted(v: 0);  // boolValue() == true
    if never {
        1
    } else if never {
        2
    } else if always {
        5
    } else {
        9
    }
}
