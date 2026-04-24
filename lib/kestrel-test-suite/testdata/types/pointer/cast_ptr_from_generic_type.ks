// test: diagnostics
// stdlib: false

module Test

func castFromGeneric[T](p: lang.ptr[T]) -> lang.ptr[lang.i8] {
    lang.cast_ptr[_, lang.i8](p)
}
