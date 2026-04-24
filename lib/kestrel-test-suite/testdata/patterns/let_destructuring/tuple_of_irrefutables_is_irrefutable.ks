// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i64, lang.i64)) -> lang.i64 {
    let (a, b) = t;
    lang.i64_add(a, b)
}
