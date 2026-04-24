// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let (a, b): (lang.i64, lang.i64) = (1, 2);
    lang.i64_add(a, b)
}
