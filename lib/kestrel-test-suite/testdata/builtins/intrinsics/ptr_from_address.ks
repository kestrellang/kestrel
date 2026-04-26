// test: diagnostics
// stdlib: false

module Test
func ptrFromAddress[T](addr: lang.i64) -> lang.ptr[T] {
    lang.ptr_from_address[T](addr)
}
