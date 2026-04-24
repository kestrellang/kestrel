// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    while true {
        return "not an lang.i64" // ERROR
    }
    0
}
