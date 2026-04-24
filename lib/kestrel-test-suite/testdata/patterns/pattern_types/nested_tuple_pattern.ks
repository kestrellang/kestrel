// test: diagnostics
// stdlib: false

module Main

func test(t: ((lang.i64, lang.i64), lang.i64)) -> lang.i64 {
    match t {
        ((a, b), c) => lang.i64_add(lang.i64_add(a, b), c)
    }
}
