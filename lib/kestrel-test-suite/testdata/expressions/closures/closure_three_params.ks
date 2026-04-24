// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64, lang.i64, lang.i64) -> lang.i64 {
    { (a, b, c) in lang.i64_add(lang.i64_add(a, b), c) }
}
