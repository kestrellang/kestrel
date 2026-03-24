// test: diagnostics
// stdlib: false

module Test

func castFromGeneric[T](p: lang.ptr[T]) -> lang.ptr[lang.i8] {
    lang.cast_ptr[lang.i8](p)
}
