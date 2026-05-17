// test: diagnostics
// stdlib: true

module Test

func matchSingle(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [x] => x,
        _ => 0
    }
}
