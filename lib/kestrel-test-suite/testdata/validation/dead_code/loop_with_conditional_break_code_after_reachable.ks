// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    loop {
        if cond {
            break;
        }
    }
    42
}
