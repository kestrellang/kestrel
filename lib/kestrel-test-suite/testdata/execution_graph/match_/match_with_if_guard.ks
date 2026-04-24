// test: diagnostics
// stdlib: false

module Main

func classify(n: lang.i64) -> lang.i64 {
    match n {
        x if lang.i64_signed_lt(x, 0) => lang.i64_sub(0, 1),
        x if lang.i64_eq(x, 0) => 0,
        x if lang.i64_signed_lt(x, 10) => 1,
        _ => 2
    }
}
