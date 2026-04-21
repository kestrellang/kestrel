// test: diagnostics
// stdlib: false

module Test

func castI8ToI32(p: lang.ptr[lang.i8]) -> lang.ptr[lang.i32] {
    lang.cast_ptr[_, lang.i32](p)
}

func castI8ToI64(p: lang.ptr[lang.i8]) -> lang.ptr[lang.i64] {
    lang.cast_ptr[_, lang.i64](p)
}

func castI8ToF32(p: lang.ptr[lang.i8]) -> lang.ptr[lang.f32] {
    lang.cast_ptr[_, lang.f32](p)
}
