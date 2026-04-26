// test: diagnostics
// stdlib: false

module Test
func isNull[T](p: lang.ptr[T]) -> lang.i1 {
    lang.ptr_is_null(p)
}
