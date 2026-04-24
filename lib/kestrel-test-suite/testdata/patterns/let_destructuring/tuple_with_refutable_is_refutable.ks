// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i64, lang.i64)) -> lang.i64 {
    let (0, b) = t; // ERROR: refutable
    b
}
