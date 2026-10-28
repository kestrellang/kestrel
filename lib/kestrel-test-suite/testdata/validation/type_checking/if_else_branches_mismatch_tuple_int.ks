// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        (1, 2) // ERROR: type mismatch: expected i64 got
    } else {
        42
    }
}
