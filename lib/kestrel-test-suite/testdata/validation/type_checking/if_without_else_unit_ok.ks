// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) {
    if cond {
        let x: lang.i64 = 42;
    }
}
