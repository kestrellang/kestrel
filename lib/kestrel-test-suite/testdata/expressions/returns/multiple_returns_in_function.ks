// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_signed_lt(x, 0) {
        return 1
    }
    if lang.i64_eq(x, 0) {
        return 0
    }
    return 1
}
