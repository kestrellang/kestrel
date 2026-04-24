// test: diagnostics
// stdlib: false

module Main

func foo(x: lang.i64) { }
func foo(x: lang.i64 = 0) { } // ERROR: duplicate
