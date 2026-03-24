// test: diagnostics
// stdlib: false

module Test
enum Result[T, E]: Prelude.BooleanConditional {
    case Ok(T)
    case Err(E)

    func asBool() -> lang.i1 {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }
}
func test(r: Result[lang.i64, lang.str]) -> lang.i64 {
    if r {
        1
    } else {
        0
    }
}
