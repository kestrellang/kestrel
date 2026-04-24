// test: diagnostics
// stdlib: true

module Main

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [.., last] => last,
        _ => 0
    }
}
