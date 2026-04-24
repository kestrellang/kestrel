// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i1, b: lang.i1) {
    while lang.i1_and(a, b) {
        ()
    }
}
