// test: diagnostics
// stdlib: false

module Main

func test() {
    let t: (lang.i64, lang.str) = (1, 2); // ERROR
}
