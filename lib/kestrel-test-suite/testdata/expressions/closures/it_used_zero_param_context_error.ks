// test: diagnostics
// stdlib: false

module Main

func test() -> () -> lang.i64 {
    { it } // ERROR: it
}
