// test: diagnostics
// stdlib: false

module Main

func transform(x: lang.i64, f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(x)
}

func test() -> lang.i64 {
    transform(5, { lang.i64_mul(it, 2) })
}
