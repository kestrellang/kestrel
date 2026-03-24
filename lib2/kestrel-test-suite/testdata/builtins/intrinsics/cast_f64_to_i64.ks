// test: diagnostics
// stdlib: false

module Test
func floatToInt(f: lang.f64) -> lang.i64 {
    lang.cast_f64_i64(f)
}
