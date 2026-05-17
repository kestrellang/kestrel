// test: diagnostics
// stdlib: false

module Main

func add((a, b): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(a, b)
}

func test() -> lang.i64 {
    add((1, 2))
}
