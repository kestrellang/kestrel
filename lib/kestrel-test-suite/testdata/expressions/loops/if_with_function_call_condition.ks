// test: diagnostics
// stdlib: false

module Main

func check() -> lang.i1 {
    true
}

func test() {
    if check() {
        ()
    }
}
