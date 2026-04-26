// test: diagnostics
// stdlib: false

module Test
func atomicSub(p: lang.ptr[lang.i64], value: lang.i64) -> lang.i64 {
    lang.atomic_sub(p, value)
}
