// test: diagnostics
// stdlib: false

module Test

@builtin(.BooleanConditional)
protocol BooleanConditional {
    func boolValue() -> lang.i1
}

enum Result[T, E]: BooleanConditional {
    case Ok(T)
    case Err(E)

    func boolValue() -> lang.i1 {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }
}
func test(r: Result[lang.i64, lang.i64]) -> lang.i64 {
    if r {
        1
    } else {
        0
    }
}
