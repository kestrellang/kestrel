// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let a = 1;
    let b = a;
    let c = b;
    c
}
