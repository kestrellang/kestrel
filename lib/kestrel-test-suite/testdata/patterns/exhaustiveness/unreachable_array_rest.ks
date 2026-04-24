// test: diagnostics
// stdlib: true

module Main

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [] => 0,
        [x] => x,
        [first, ..rest, last] => lang.i64_add(first, last),
        [..] => 0 // WARN: unreachable
    }
}
