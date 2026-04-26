// test: diagnostics
// stdlib: true

module Test
func integer() -> lang.i64 { 42 }
func floating() -> lang.f64 { 3.14 }
func string() -> lang.str { "hello" }
func boolean() -> lang.i1 { true }
func sequence() -> [lang.i64] { [1, 2, 3] }
func pair() -> (lang.i64, lang.i64) { (1, 2) }
