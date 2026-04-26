// test: diagnostics
// stdlib: false

module Main

func test() {
    sibling: while true {
        break;
    }
    while true {
        break sibling; // ERROR: undeclared label
    }
}
