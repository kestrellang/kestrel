// test: diagnostics
// stdlib: false

module Test

type Bad = lang.ptr[]; // ERROR: type argument
