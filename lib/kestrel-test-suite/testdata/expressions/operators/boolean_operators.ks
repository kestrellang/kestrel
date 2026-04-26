// test: diagnostics
// stdlib: false

module Main

func testAnd() -> lang.i1 {
    lang.i1_and(true, false)
}

func testOr() -> lang.i1 {
    lang.i1_or(true, false)
}

func testNot() -> lang.i1 {
    lang.i1_not(true)
}
