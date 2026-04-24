// test: diagnostics
// stdlib: false

module Main

func shouldContinue() -> lang.i1 {
    false
}

func test() {
    while shouldContinue() {
        ()
    }
}
