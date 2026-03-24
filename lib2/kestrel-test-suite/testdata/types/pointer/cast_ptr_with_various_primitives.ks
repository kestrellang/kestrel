// test: diagnostics
// stdlib: false

module Test

func testI8() -> lang.ptr[lang.i8] {
    lang.cast_ptr[lang.i8](lang.ptr_null())
}

func testI16() -> lang.ptr[lang.i16] {
    lang.cast_ptr[lang.i16](lang.ptr_null())
}

func testI32() -> lang.ptr[lang.i32] {
    lang.cast_ptr[lang.i32](lang.ptr_null())
}

func testF32() -> lang.ptr[lang.f32] {
    lang.cast_ptr[lang.f32](lang.ptr_null())
}

func testF64() -> lang.ptr[lang.f64] {
    lang.cast_ptr[lang.f64](lang.ptr_null())
}
