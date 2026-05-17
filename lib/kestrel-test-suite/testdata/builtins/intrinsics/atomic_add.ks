// test: diagnostics
// stdlib: false

module Test
func atomicAdd(p: lang.ptr[lang.i64], value: lang.i64) -> lang.i64 {
    lang.atomic_add(p, value)
}
