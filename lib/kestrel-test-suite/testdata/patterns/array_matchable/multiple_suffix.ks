// test: diagnostics
// stdlib: true

module Test

func lastTwo(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [.., y, z] => lang.i64_add(y, z),
        _ => 0
    }
}
