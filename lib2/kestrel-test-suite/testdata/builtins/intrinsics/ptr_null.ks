// test: diagnostics
// stdlib: false

module Test
func getNullPtr[T]() -> lang.ptr[T] {
    lang.ptr_null[T]()
}
