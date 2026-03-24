// test: diagnostics
// stdlib: false

module Test
func i16ToI64(s: lang.i16) -> lang.i64 {
    lang.cast_i16_i64(s)
}
func i64ToI16(l: lang.i64) -> lang.i16 {
    lang.cast_i64_i16(l)
}
