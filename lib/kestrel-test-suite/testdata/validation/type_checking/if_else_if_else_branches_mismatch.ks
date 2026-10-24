// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 1) {
        10
    } else if lang.i64_eq(x, 2) {
        "twenty" // ERROR: type 'i64' does not accept string literal
    } else {
        30
    }
}
