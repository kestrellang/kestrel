// test: diagnostics
// stdlib: true

module Main

func test() -> lang.i64 {
    return [1, 2, 3] // ERROR
}
