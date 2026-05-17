// test: diagnostics
// stdlib: true

module Test

func matchEmpty(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [] => 1,
        _ => 0
    }
}
