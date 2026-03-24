// test: diagnostics
// stdlib: false

module Test

struct Holder {
    let ptr: lang.ptr[lang.i8]
}
func wrap(s: lang.str) -> Holder {
    Holder(ptr: s.unsafePtr())
}
