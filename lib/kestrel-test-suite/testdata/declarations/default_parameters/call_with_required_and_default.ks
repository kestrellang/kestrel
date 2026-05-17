// test: diagnostics
// stdlib: false

module Main

func compute(numerator: lang.i64, denominator: lang.i64 = 1) -> lang.i64 {
    lang.i64_signed_div(numerator, denominator)
}

func test() -> lang.i64 {
    compute(42)
}
