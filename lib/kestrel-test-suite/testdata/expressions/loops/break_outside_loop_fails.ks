// test: diagnostics
// stdlib: false

module Main

func test() {
    break; // ERROR: outside of loop
}
