// test: diagnostics
// stdlib: false

module Main

func getNumber() -> lang.i64 {
    42
}

func test() -> lang.i64 {
    getNumber()
}
