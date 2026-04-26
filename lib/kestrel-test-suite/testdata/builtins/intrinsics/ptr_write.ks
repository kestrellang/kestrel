// test: diagnostics
// stdlib: false

module Test
func writePtr(p: lang.ptr[lang.i64], value: lang.i64) {
    lang.ptr_write(p, value)
}
