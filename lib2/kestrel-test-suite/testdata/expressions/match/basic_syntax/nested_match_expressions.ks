// test: diagnostics
// stdlib: false

module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    match a {
        true => match b {
            true => 1,
            false => 2
        },
        false => 3
    }
}
