// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    if true {
        let x: lang.i64 = 42;
        x
    }
    x // ERROR: undefined
}
