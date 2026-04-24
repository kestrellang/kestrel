// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.i64 {
    { (x) in lang.i64_add(x, 1) }
}
