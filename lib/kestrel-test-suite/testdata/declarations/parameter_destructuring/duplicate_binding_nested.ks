// test: diagnostics
// stdlib: false

module Main

func bad((x, (x, y)): (lang.i64, (lang.i64, lang.i64))) -> lang.i64 { // ERROR: duplicate
    x
}
