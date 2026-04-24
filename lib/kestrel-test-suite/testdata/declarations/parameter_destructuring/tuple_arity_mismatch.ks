// test: diagnostics
// stdlib: false

module Main

func bad((a, b, c): (lang.i64, lang.i64)) -> lang.i64 { // ERROR:
    a
}
