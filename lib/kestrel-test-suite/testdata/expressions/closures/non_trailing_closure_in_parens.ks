// test: diagnostics
// stdlib: false

module Main

func apply(f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(10)
}

func test() -> lang.i64 {
    apply({ lang.i64_mul(it, 2) })
}
