// test: diagnostics
// stdlib: true

module Main

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, second, ..] => lang.i64_add(first, second),
        [only] => only,
        [] => 0
    }
}
