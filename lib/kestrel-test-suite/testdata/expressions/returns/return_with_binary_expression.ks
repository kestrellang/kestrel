// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i64, b: lang.i64) -> lang.i64 {
    return lang.i64_add(lang.i64_mul(a, b), 1)
}
