// test: diagnostics
// stdlib: true

module Test

func getFirst(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, ..] => first,
        _ => 0
    }
}
