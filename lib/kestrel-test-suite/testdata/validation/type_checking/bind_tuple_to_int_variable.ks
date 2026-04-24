// test: diagnostics
// stdlib: false

module Main

func test() {
    let x: lang.i64 = (1, 2, 3); // ERROR
}
