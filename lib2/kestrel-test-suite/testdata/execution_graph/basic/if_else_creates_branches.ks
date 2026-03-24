// test: diagnostics
// stdlib: false

module Main
func abs(x: lang.i64) -> lang.i64 {
    if lang.i64_signed_lt(x, 0) { lang.i64_neg(x) } else { x }
}
