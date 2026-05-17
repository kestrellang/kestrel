// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond { // ERROR
        (1, 2)
    } else {
        42 // ERROR
    }
}
