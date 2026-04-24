// test: diagnostics
// stdlib: true

module Test

func endpoints(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, .., last] => lang.i64_add(first, last),
        _ => 0
    }
}
