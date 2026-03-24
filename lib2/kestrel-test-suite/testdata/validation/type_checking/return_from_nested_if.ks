// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            return "wrong" // ERROR
        }
        1
    } else {
        2
    }
}
