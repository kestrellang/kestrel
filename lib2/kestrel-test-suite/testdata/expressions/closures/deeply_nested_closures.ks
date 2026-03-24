// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> (lang.i64) -> (lang.i64) -> lang.i64 {
    { (a) in { (b) in { (c) in lang.i64_add(lang.i64_add(a, b), c) } } } // ERROR: cannot return a closure that captures variables
}
