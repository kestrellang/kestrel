// test: diagnostics
// stdlib: false

module Main

func test(b: lang.i1) -> lang.i64 {
    let result = match b {
        true => 42,
        false => 0
    };
    result
}
