// test: diagnostics
// stdlib: false

module Test

struct DoublePtr {
    let ptr: lang.ptr[lang.ptr[lang.i64]]
}
