// test: diagnostics
// stdlib: false

module Main

func test() {
    while true {
        let x: lang.i64 = 42;
        break;
    }
    x // ERROR: undefined
}
