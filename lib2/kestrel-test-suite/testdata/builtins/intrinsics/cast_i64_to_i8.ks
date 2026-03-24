// test: diagnostics
// stdlib: false

module Test
func longToByte(l: lang.i64) -> lang.i8 {
    lang.cast_i64_i8(l)
}
