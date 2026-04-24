// test: diagnostics
// stdlib: false

module Main

func test() {
    let t: (lang.i64, lang.i64) = (1, 2, 3); // ERROR: type mismatch
}
