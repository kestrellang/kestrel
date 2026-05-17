// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    return (1, 2) // ERROR
}
