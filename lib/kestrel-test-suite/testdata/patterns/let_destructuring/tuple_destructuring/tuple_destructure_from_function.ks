// test: diagnostics
// stdlib: false

module Main

func pair() -> (lang.i64, lang.i64) {
    (1, 2)
}

func test() -> lang.i64 {
    let (a, b) = pair();
    lang.i64_add(a, b)
}
