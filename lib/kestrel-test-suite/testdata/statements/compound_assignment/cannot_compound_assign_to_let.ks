// test: diagnostics
// stdlib: true

module Main

func test() {
    let x: Int = 5;
    x += 1; // ERROR:
}
