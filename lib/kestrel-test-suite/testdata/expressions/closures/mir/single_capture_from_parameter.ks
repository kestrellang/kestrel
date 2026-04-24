// test: diagnostics
// stdlib: false

module Test

func test(n: lang.i64) -> () -> lang.i64 {
    { lang.i64_add(n, 1) } // ERROR: cannot return a closure that captures variables
}
