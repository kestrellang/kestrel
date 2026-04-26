// test: diagnostics
// stdlib: true

module Main

func test() {
    var a: Int = 1;
    var b: Int = 2;
    a += b += 1; // ERROR:
}
