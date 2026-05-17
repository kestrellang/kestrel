// test: diagnostics
// stdlib: false

module Main

func createPoint(x: lang.i64 = 0, y: lang.i64 = 0) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    createPoint(10)
}
