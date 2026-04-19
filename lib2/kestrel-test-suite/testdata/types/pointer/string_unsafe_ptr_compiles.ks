// test: diagnostics
// stdlib: false

module Test

func getPtr(s: lang.str) -> lang.ptr[lang.i8] {
    lang.str_ptr(s)
}
