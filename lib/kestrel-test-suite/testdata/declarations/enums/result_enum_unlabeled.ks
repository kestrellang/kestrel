// test: diagnostics
// stdlib: false

module Test

enum Result[T, E] {
    case Ok(T)
    case Err(E)
}

func getValue(r: Result[lang.i64, lang.str]) -> lang.i64 {
    match r {
        .Ok(v) => v,
        .Err(_e) => 0
    }
}
