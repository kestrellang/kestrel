// test: diagnostics
// stdlib: false

module Main

func bothTrue() -> lang.i1 {
    lang.i1_and(true, true)
}

func eitherTrue() -> lang.i1 {
    lang.i1_or(true, false)
}

func negate() -> lang.i1 {
    lang.i1_not(true)
}
