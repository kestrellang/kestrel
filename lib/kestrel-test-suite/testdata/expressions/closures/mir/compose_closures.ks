// test: diagnostics
// stdlib: false

module Main

func compose(f: (lang.i64) -> lang.i64, g: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
    f(g(x))
}
