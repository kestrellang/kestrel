// test: diagnostics
// stdlib: false
//
// E645: only a computed property can be `unsafe`; unsafe stored fields are
// not supported.

module Test

struct S {
    unsafe var raw: lang.i64 // ERROR: only a computed property can be `unsafe`
    unsafe var computed: lang.i64 { 0 }
}
