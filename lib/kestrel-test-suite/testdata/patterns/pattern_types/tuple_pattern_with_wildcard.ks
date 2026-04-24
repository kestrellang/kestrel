// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    match t {
        (first, _, last) => lang.i64_add(first, last)
    }
}
