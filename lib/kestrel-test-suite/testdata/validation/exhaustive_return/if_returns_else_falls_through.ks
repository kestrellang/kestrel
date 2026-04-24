// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        return 1
    } else {
        let x: lang.i64 = 2;
    }
} // ERROR
