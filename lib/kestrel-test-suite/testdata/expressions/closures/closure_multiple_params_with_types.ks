// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64, lang.i64) -> lang.i64 {
    { (x: lang.i64, y: lang.i64) in lang.i64_add(x, y) }
}
