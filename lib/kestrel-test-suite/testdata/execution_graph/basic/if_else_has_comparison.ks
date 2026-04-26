// test: diagnostics
// stdlib: false

module Main
func max(a: lang.i64, b: lang.i64) -> lang.i64 {
    if lang.i64_signed_gt(a, b) { a } else { b }
}
