// test: diagnostics
// stdlib: false

module Main

func test() -> () -> lang.i64 {
    let x = 10;
    { lang.i64_add(x, 1) } // ERROR: cannot return a closure that captures variables
}
