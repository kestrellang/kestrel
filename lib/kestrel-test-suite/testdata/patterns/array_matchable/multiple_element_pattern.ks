// test: diagnostics
// stdlib: true

module Test

func sum3(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [a, b, c] => lang.i64_add(a, lang.i64_add(b, c)),
        _ => 0
    }
}
