// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    if true {
        1
    } else {
        let y: lang.i64 = 10;
        y
    }
    y // ERROR: undefined
}
