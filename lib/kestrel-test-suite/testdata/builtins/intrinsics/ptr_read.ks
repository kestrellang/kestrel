// test: diagnostics
// stdlib: false

module Test
func readPtr(p: lang.ptr[lang.i64]) -> lang.i64 {
    lang.ptr_read(p)
}
