// test: diagnostics
// stdlib: false

module Main

func test(multiplier: lang.i64) -> (lang.i64) -> lang.i64 {
    { lang.i64_mul(it, multiplier) } // ERROR: cannot return a closure that captures variables
}
