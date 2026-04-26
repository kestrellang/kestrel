// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    loop {
        return 1;
    }
    42 // WARN: unreachable
}
