// test: diagnostics
// stdlib: false

module Test
func readPtr(p: lang.ptr[lang.i64]) -> lang.i64 {
    unsafe lang.ptr_read(p)
}
