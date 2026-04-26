// test: diagnostics
// stdlib: false

module Main

func test() {
    outermost: while true {
        while true {
            loop {
                break outermost;
            }
        }
    }
}
