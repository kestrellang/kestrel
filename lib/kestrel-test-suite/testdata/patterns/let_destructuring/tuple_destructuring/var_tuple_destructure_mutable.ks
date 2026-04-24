// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var (a, b) = (1, 2);
    a = 10;
    b = 20;
    lang.i64_add(a, b)
}
