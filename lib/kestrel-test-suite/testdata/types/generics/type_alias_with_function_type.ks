// test: diagnostics
// stdlib: false

module Test

type Transform = (lang.i64) -> lang.i64
func apply(f: Transform, x: lang.i64) -> lang.i64 { f(x) }
