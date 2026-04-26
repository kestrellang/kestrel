// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    return "hello" // ERROR
}
