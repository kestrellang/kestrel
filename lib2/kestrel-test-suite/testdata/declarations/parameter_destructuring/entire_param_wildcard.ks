// test: diagnostics
// stdlib: false

module Main

func ignore(_: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    ignore(100)
}
