// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    x = "hello"; // ERROR
}
