// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let f = { ((a, a): (lang.i64, lang.i64)) in a }; // ERROR: duplicate
    f((1, 2))
}
