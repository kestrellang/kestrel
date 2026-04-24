// test: diagnostics
// stdlib: false

module Main

func test() {
    for x in 42 { // ERROR: Iterable
        ()
    }
}
