// test: diagnostics
// stdlib: false

module Test
func ptrToAddress[T](p: lang.ptr[T]) -> lang.i64 {
    lang.ptr_to_address(p)
}
