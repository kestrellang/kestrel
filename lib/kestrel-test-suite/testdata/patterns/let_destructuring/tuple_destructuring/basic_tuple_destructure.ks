// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let (a, b) = (1, 2);
    lang.i64_add(a, b)
}
