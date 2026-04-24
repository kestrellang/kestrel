// test: diagnostics
// stdlib: false

module Main

func first((a, _): (lang.i64, lang.i64)) -> lang.i64 {
    a
}

func test() -> lang.i64 {
    first((42, 100))
}
