// test: diagnostics
// stdlib: false

module Test
func alignOfI64() -> lang.i64 {
    lang.alignof[lang.i64]()
}
