// test: diagnostics
// stdlib: false

module Main

func test() {
    outer: loop {
        loop {
            break outer;
        }
    }
}
