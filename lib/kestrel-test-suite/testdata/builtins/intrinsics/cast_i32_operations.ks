// test: diagnostics
// stdlib: false

module Test
func i32ToI64(i: lang.i32) -> lang.i64 {
    lang.cast_i32_i64(i)
}
func i64ToI32(l: lang.i64) -> lang.i32 {
    lang.cast_i64_i32(l)
}
