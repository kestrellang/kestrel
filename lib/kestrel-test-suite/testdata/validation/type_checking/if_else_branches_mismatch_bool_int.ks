// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond { // ERROR
        true
    } else {
        42 // ERROR
    }
}
