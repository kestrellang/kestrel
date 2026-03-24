// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.i64 {
    { (x, y) in lang.i64_add(x, y) } // ERROR:
}
