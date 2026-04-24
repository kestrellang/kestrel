// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    if false {
        1
    }
    if false {
        2
    }
    if true {
        3
    } else {
        4
    }
}
