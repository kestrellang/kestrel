// test: diagnostics
// stdlib: false

module Main

func test() {
    continue; // ERROR: outside of loop
}
