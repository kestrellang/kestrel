// test: diagnostics
// stdlib: false

module Main

func test(b: lang.i1) -> lang.i64 {
    match b {
        true => return 42,
        false => 0
    }
}
