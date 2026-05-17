// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    while cond { // ERROR: type mismatch
        return 1
    }
}
