// test: diagnostics
// stdlib: false

module Main

func test() {
    while true {
        break nonexistent; // ERROR: undeclared label
    }
}
