// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    loop {
        return "not an lang.i64" // ERROR
    }
}
