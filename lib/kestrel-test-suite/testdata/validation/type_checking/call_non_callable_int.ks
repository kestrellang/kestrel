// test: diagnostics
// stdlib: false

module Main
func test() {
    let x = 42;
    x(); // ERROR
}
