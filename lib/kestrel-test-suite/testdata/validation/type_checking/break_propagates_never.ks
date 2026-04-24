// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    loop {
        if true {
            break
        } else {
            return 42
        }
    }
    0
}
