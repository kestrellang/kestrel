// test: diagnostics
// stdlib: false

module Main

func test() {
    while true {
        break inner; // ERROR: undeclared label
        inner: loop {
            break;
        }
    }
}
