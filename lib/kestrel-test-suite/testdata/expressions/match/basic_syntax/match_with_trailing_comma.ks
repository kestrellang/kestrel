// test: diagnostics
// stdlib: false

module Main

func test(b: lang.i1) -> lang.i64 {
    match b {
        true => 1,
        false => 0,
    }
}
