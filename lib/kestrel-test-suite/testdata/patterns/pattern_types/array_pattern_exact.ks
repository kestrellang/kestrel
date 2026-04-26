// test: diagnostics
// stdlib: true

module Main

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [a, b] => lang.i64_add(a, b),
        _ => 0
    }
}
