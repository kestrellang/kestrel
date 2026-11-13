// test: diagnostics
// stdlib: false
//
// The `lang` intrinsics that read, write or drop through a pointer are
// unsafe. Making, offsetting and comparing pointers is not.

module Test

func f(p: lang.ptr[lang.i64]) -> lang.i64 {
    let q = lang.ptr_offset(p, 1);
    let isNull = lang.ptr_is_null(q);
    lang.ptr_write(q, 5); // ERROR: 'ptr_write' is unsafe and requires an `unsafe` expression or block
    lang.ptr_read(q) // ERROR: 'ptr_read' is unsafe and requires an `unsafe` expression or block
}

func g(p: lang.ptr[lang.i64]) -> lang.i64 {
    unsafe lang.ptr_write(p, 5);
    unsafe lang.ptr_read(p)
}
