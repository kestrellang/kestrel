// test: diagnostics
// stdlib: false

module Main

func add(x: lang.i64, y: lang.i64) -> lang.i64 {
    lang.i64_add(x, y)
}

func compute(a: lang.i64, b: lang.i64, c: lang.i64) -> lang.i64 {
    lang.i64_add(lang.i64_mul(a, b), c)
}
