// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    loop { // ERROR: type mismatch
        break
    }
}
