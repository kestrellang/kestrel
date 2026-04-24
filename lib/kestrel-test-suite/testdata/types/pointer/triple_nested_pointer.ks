// test: diagnostics
// stdlib: false

module Test

type PtrPtrPtr = lang.ptr[lang.ptr[lang.ptr[lang.i64]]];
