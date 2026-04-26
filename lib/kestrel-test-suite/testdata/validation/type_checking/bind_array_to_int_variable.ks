// test: diagnostics
// stdlib: true

module Main

func test() {
    let x: lang.i64 = [1, 2, 3]; // ERROR
}
