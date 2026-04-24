// test: diagnostics
// stdlib: false

module Main

func test() {
    if true {
        break; // ERROR: outside of loop
    }
}
