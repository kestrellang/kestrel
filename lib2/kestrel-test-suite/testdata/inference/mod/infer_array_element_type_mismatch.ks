// test: diagnostics
// stdlib: true

module Main

func test() {
    let arr = [1, 2, 3];
    let x: [lang.str] = arr; // ERROR: does not conform to protocol
}
