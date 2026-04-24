// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i1) -> lang.i64 {
    if x {
        return 1
    } else {
        return 0
    }
}
