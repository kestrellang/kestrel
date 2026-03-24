// test: diagnostics
// stdlib: false

module Main

func test() {
    let arr: [lang.i64] = [1, "two", true, 4.0]; // ERROR
}
