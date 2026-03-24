// test: diagnostics
// stdlib: false

module Test

struct StringView {
    let ptr: lang.ptr[lang.i8]
    let len: lang.i64
}
func makeView(s: lang.str) -> StringView {
    StringView(ptr: s.unsafePtr(), len: s.length())
}
