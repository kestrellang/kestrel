// test: diagnostics
// stdlib: false

module Test
func assertNonZero(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 0) {
        lang.panic_unwind("value must be non-zero")
    } else {
        x
    }
}
