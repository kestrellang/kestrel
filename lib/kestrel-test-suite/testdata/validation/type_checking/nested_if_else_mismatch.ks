// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            42
        } else {
            "wrong" // ERROR
        }
    } else {
        0
    }
}
