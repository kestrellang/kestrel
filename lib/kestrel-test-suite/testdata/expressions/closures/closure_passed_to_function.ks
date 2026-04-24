// test: diagnostics
// stdlib: false

module Main

func apply(x: lang.i64, f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(x)
}

func test() -> lang.i64 {
    apply(10, { lang.i64_add(it, 1) })
}
