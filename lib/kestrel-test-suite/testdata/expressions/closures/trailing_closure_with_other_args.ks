// test: diagnostics
// stdlib: false

module Main

func fold(initial: lang.i64, f: (lang.i64, lang.i64) -> lang.i64) -> lang.i64 {
    f(initial, 10)
}

func test() -> lang.i64 {
    fold(0) { (acc, n) in lang.i64_add(acc, n) }
}
