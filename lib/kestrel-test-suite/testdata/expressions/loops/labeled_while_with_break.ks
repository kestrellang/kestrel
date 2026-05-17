// test: diagnostics
// stdlib: false

module Main

func test() {
    outer: while true {
        while true {
            break outer;
        }
    }
}
