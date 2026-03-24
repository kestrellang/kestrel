// test: diagnostics
// stdlib: false

module Test

func convert(x: lang.i64) -> lang.str { "lang.i64" }
func convert(x: lang.f64) -> lang.str { "float" } // ERROR: duplicate function signature
