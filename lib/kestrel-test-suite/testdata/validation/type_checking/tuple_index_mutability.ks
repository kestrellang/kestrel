// test: diagnostics
// stdlib: false

module Main

func test() {
    var t = (1, 2);
    t.0 = 10;
    t.1 = 20;
}
