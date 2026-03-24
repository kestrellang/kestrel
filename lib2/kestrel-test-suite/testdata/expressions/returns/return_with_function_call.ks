// test: diagnostics
// stdlib: false

module Main

func helper() -> lang.i64 {
    42
}

func test() -> lang.i64 {
    return helper()
}
