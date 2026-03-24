// test: diagnostics
// stdlib: false

module Test

func test() -> lang.ptr[lang.i64] {
    lang.cast_ptr[lang.i64](lang.ptr_null())
}
