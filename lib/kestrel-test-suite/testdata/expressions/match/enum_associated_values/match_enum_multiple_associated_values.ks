// test: diagnostics
// stdlib: false

module Main

enum Result[T, E] {
    case Ok(value: T)
    case Err(error: E)
}

func test(r: Result[lang.i64, lang.str]) -> lang.i64 {
    match r {
        .Ok(value) => value,
        .Err(error) => 0
    }
}
