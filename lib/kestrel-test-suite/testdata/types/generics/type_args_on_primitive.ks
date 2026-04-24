// test: diagnostics
// stdlib: false

module Test

type Bad = lang.i64[lang.str]; // ERROR: does not accept type arguments
