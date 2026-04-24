// test: diagnostics
// stdlib: false

module Main

func add(x: lang.i64, y: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    add(1, 2)
}
