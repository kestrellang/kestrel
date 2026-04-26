// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.str = "hello";
    x = true; // ERROR
}
