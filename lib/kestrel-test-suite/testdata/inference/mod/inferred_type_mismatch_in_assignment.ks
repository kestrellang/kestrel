// test: diagnostics
// stdlib: false

module Main

func test() {
    var x = "hello";
    x = 42 // ERROR: type mismatch
}
