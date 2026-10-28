// test: diagnostics
// stdlib: false

module Main

func test(multiplier: lang.i64) -> (lang.i64) -> lang.i64 {
    { lang.i64_mul(it, multiplier) } // ERROR(E494)
}
