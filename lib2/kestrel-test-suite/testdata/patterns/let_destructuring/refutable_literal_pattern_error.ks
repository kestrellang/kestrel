// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    let 42 = x; // ERROR: refutable
    42
}
