// test: diagnostics
// stdlib: false

module Main

func check() -> lang.i1 {
    lang.i1_or(lang.i1_and(true, false), true)
}
