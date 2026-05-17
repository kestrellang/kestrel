// test: diagnostics
// stdlib: false

module Main

func nested(((a, b), c): ((lang.i64, lang.i64), lang.i64)) -> lang.i64 {
    lang.i64_add(lang.i64_add(a, b), c)
}

func test() -> lang.i64 {
    nested(((1, 2), 3))
}
