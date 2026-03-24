// test: diagnostics
// stdlib: false

module Main

func double(x: lang.i64) -> lang.i64 {
    lang.i64_add(x, x)
}

func test() {
    double((1, 2)); // ERROR
}
