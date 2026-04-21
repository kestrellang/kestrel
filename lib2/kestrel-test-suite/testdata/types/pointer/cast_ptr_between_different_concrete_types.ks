// test: diagnostics
// stdlib: false

module Test

func castI32ToI8(p: lang.ptr[lang.i32]) -> lang.ptr[lang.i8] {
    lang.cast_ptr[_, lang.i8](p)
}
