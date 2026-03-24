// test: diagnostics
// stdlib: false

module Main

func test() -> () -> lang.i64 {
    let a = 1;
    let b = 2;
    let c = 3;
    { lang.i64_add(lang.i64_add(a, b), c) } // ERROR: cannot return a closure that captures variables
}
