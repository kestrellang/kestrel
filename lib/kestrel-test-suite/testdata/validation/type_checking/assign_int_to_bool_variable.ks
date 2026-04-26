// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i1 = true;
    x = 42; // ERROR
}
