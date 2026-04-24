// test: diagnostics
// stdlib: false

module Main

func combine(a: lang.i64, b: lang.i64, f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(lang.i64_add(a, b))
}

func test() -> lang.i64 {
    combine(1, 2) { lang.i64_mul(it, 2) }
}
