// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i1, b: lang.i1, c: lang.i1) -> lang.i64 {
    if a {
        if b {
            if c {
                42
            } else {
                "wrong" // ERROR
            }
        } else {
            0
        }
    } else {
        0
    }
}
