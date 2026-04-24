// test: diagnostics
// stdlib: false

module Test

type Bad = lang.ptr[lang.i64, lang.str, lang.i1]; // ERROR: too many type arguments
