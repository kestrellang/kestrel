// test: diagnostics
// stdlib: false

module Main

enum Wrapper[T] {
    case Value(inner: T)
}

func test(w: Wrapper[lang.i64]) -> lang.i64 {
    let .Value(inner) = w;
    inner
}
