// test: diagnostics
// stdlib: true

module Main

func test() {
    let arr = [1, 2, 3]; // ERROR: type mismatch
    let x: [lang.str] = arr;
}
