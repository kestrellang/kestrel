// test: diagnostics
// stdlib: false

module Test

func castToGeneric[T](p: lang.ptr[lang.i8]) -> lang.ptr[T] {
    lang.cast_ptr[_, T](p)
}
