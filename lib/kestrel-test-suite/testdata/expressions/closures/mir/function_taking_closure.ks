// test: diagnostics
// stdlib: false

module Main

func apply(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
    f(x)
}
