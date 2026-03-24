// test: diagnostics
// stdlib: false

module Main

func test() {
    myloop: while true {
        break mylooop; // ERROR: undeclared label
    }
}
