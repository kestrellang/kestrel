// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64, lang.str) -> lang.i64 {
    { (x: lang.i64, y) in x }
}
