// test: diagnostics
// stdlib: false

module Main

func add(point (x, y): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    add(point: (1, 2))
}
