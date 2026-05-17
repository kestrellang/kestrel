// test: diagnostics
// stdlib: false

module Main

func invalid(a: lang.i64 = 0, b: lang.i64) { } // ERROR: required parameter
