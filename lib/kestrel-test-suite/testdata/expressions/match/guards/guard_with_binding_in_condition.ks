// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.str {
    match x {
        n if lang.i64_signed_gt(n, 100) => "big",
        n if lang.i64_signed_gt(n, 10) => "medium",
        _ => "small"
    }
}
