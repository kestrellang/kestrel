// test: diagnostics
// stdlib: false

module Test

func test() -> (lang.i64) -> (lang.i64) -> lang.i64 {
    { (x) in { (y) in lang.i64_add(x, y) } } // ERROR: cannot return a closure that captures variables
}
