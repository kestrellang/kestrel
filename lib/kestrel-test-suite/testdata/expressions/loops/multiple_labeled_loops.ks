// test: diagnostics
// stdlib: false

module Main

func test() {
    outer: loop {
        inner: loop {
            break inner;
        }
        break outer;
    }
}
