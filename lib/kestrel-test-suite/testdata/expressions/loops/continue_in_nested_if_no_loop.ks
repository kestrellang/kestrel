// test: diagnostics
// stdlib: false

module Main

func test() {
    if true {
        if false {
            continue; // ERROR: outside of loop
        }
    }
}
