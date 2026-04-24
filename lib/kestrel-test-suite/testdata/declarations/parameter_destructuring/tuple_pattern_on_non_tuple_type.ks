// test: diagnostics
// stdlib: false

module Main

func bad((a, b): lang.i64) -> lang.i64 { // ERROR:
    a
}
