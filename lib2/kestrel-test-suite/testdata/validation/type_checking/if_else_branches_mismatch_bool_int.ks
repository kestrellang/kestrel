// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        true // ERROR
    } else {
        42
    }
}
