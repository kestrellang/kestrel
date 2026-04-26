// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    { (x: lang.i64) in x }(1, 2) // ERROR:
}
