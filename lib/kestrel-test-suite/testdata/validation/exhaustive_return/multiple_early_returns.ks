// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 0) {
        return 0
    }
    if lang.i64_eq(x, 1) {
        return 1
    }
    x
}
