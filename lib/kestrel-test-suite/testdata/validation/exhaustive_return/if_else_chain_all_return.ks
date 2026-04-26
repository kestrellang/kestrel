// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 1) {
        return 10
    } else if lang.i64_eq(x, 2) {
        return 20
    } else {
        return 0
    }
}
