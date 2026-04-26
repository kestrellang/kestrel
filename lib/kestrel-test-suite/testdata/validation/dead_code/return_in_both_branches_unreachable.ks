// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        return 1;
    } else {
        return 2;
    }
    42 // WARN: unreachable
}
