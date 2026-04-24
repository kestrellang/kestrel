// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i64, lang.i64)) -> lang.i64 {
    match t {
        (a, b) => lang.i64_add(a, b)
    }
}
