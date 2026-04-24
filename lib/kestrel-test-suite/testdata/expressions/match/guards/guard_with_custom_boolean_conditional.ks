// test: diagnostics
// stdlib: false

module Main

@builtin(.BooleanConditional)
protocol BooleanConditional {
    func boolValue() -> lang.i1
}

struct Bool {
    let value: lang.i1
}

extend Bool: BooleanConditional {
    func boolValue() -> lang.i1 {
        self.value
    }
}

func test(x: lang.i64) -> lang.str {
    let condition = Bool(value: lang.i64_signed_gt(x, 0));
    match x {
        n if condition => "positive",
        _ => "non-positive"
    }
}
