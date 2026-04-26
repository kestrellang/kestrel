// test: diagnostics
// stdlib: false

module Main

func combine(a: lang.i64, b: lang.i64, c: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    combine(1, 2, 3)
}
