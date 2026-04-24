// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if lang.i1_and(a, b) {
        1
    } else {
        0
    }
}
