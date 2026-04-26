// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    while cond {
        return 1
    }
} // ERROR
