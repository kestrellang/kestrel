// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.i64 {
    { (it) in lang.i64_mul(it, 2) }
}
