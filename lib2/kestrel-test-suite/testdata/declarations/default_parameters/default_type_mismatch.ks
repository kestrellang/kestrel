// test: diagnostics
// stdlib: false

module Main

func bad(x: lang.i64 = "not an int") { } // ERROR: type mismatch
