// test: diagnostics
// stdlib: false

module Main

func both((a, b): (lang.i64, lang.i64), (c, d): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(lang.i64_add(a, b), lang.i64_add(c, d))
}

func test() -> lang.i64 {
    both((1, 2), (3, 4))
}
