// test: diagnostics
// stdlib: false

module Main

func classify(n: lang.i64) -> lang.i64 {
    if lang.i64_signed_lt(n, 0) {
        return lang.i64_sub(0, 1)
    }
    if lang.i64_eq(n, 0) {
        return 0
    }
    if lang.i64_signed_lt(n, 10) {
        return 1
    }
    if lang.i64_signed_lt(n, 100) {
        return 2
    }
    3
}
