// test: diagnostics
// stdlib: true

module Main

func test() {
    let arr: [lang.i64] = [1, "two", 3]; // ERROR
}
