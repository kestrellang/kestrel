// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let (a, b): (lang.i64, lang.i64) = (1, "hello"); // ERROR: type
    a
}
