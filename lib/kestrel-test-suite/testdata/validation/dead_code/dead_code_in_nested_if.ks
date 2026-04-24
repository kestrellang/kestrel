// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            return 1;
        } else {
            return 2;
        }
        let x: lang.i64 = 3; // WARN: unreachable
    }
    0
}
