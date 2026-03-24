// test: diagnostics
// stdlib: false

module Main

func makeAdder(n: lang.i64) -> (lang.i64) -> lang.i64 {
    { (x: lang.i64) in lang.i64_add(x, n) } // ERROR: cannot return a closure that captures variables
}
