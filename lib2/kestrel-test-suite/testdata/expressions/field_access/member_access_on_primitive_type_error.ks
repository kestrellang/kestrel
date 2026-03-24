// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    x.foo // ERROR: cannot access member on type
}
