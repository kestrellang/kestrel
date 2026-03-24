// test: diagnostics
// stdlib: false

module Main

func try_mutate((a, b): (lang.i64, lang.i64)) -> lang.i64 {
    a = 10;  // ERROR: immutable
    a
}
