// test: diagnostics
// stdlib: false

module Test

struct MultiPtr {
    let intPtr: lang.ptr[lang.i64]
    let strPtr: lang.ptr[lang.str]
    let boolPtr: lang.ptr[lang.i1]
}
