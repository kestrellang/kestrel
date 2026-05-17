// test: diagnostics
// stdlib: false

module Main

func bad((a, a): (lang.i64, lang.i64)) -> lang.i64 { // ERROR: duplicate
    a
}
