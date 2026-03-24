// test: diagnostics
// stdlib: false

module Main

func test() {
    let arr: [lang.i64] = [1, 2, true]; // ERROR
}
