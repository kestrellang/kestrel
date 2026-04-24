// test: diagnostics
// stdlib: false

module Test

func double(x: lang.i64) -> lang.i64 { lang.i64_mul(x, 2) }
func negate(x: lang.i64) -> lang.i64 { lang.i64_neg(x) }
