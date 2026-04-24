// test: diagnostics
// stdlib: false

module Test

struct MultiPtr {
    let a: lang.ptr[lang.i64]
    let b: lang.ptr[lang.i1]
    let c: lang.ptr[lang.str]
}
