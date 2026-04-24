// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let first = { ((a, _): (lang.i64, lang.i64)) in a };
    first((42, 100))
}
