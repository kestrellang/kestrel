// test: diagnostics
// stdlib: true

module Main

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, ..] => first,
        [] => 0
    }
}
