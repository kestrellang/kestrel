// test: diagnostics
// stdlib: false

module Main

func simple(x: lang.i64, y: lang.i64) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    simple(1, 2)
}
