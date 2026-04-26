// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let f = { ((a, b, c): (lang.i64, lang.i64)) in a }; // ERROR:
    f((1, 2))
}
