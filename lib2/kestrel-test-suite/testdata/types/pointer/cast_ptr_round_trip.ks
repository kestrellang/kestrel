// test: diagnostics
// stdlib: false

module Test

func roundTrip(p: lang.ptr[lang.i32]) -> lang.ptr[lang.i32] {
    var bytes = lang.cast_ptr[lang.i8](p);
    lang.cast_ptr[lang.i32](bytes)
}
