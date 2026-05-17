// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let (var a, b) = (1, 2);
    a = 10;
    lang.i64_add(a, b)
}
