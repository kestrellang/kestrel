// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.str {
    match x { // ERROR: exhaustive
        n if lang.i64_signed_gt(n, 0) => "positive",
        n if lang.i64_signed_lt(n, 0) => "negative"
    }
}
