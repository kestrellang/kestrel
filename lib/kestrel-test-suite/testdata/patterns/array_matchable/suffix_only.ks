// test: diagnostics
// stdlib: true

module Test

func getLast(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [.., last] => last,
        _ => 0
    }
}
