// test: diagnostics
// stdlib: false

module Test
func intToFloat(i: lang.i64) -> lang.f64 {
    lang.cast_i64_f64(i)
}
