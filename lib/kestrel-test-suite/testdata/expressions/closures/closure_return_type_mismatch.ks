// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.str {
    { (x) in lang.i64_mul(x, 2) } // ERROR:
}
