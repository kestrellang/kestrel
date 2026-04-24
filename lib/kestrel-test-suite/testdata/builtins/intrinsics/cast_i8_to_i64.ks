// test: diagnostics
// stdlib: false

module Test
func byteToLong(b: lang.i8) -> lang.i64 {
    lang.cast_i8_i64(b)
}
