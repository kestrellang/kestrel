// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    var y: lang.str = "hello";
    x = 42;
    y = x // ERROR: type mismatch
}
