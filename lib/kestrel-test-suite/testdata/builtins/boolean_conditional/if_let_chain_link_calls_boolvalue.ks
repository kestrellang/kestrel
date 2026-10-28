// test: execution
// stdlib: true
// expect-exit: 6

// G18 position coverage: the non-binding link of an `if let p = e, cond`
// chain. The chain lowers to nested `HirExpr::If`s, so the bare `cond` link
// reaches `lower_if` like any other condition.

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
    let never = Inverted(v: 9);
    let slot = Slot.Some(value: 1);
    // The pattern binds, but `never.boolValue()` is false -> else. A raw
    // branch sees the nonzero payload and takes the then arm.
    if let .Some(x) = slot, never {
        x
    } else {
        6
    }
}
