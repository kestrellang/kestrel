// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    while true { // ERROR: type mismatch
        loop {
            let inner: lang.i64 = 42;
            break;
        }
        inner // ERROR: undefined
    }
}
