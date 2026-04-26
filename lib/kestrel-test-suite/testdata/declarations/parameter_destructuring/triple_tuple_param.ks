// test: diagnostics
// stdlib: false

module Main

func first((x, y, z): (lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    x
}

func test() -> lang.i64 {
    first((1, 2, 3))
}
