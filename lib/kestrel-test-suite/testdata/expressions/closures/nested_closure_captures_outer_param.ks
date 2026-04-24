// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let f: (lang.i64) -> (lang.i64) -> lang.i64 = { (x) in { (y) in lang.i64_add(x, y) } }; // ERROR: cannot return a closure that captures variables
    let add10 = f(10);
    add10(5)
}
