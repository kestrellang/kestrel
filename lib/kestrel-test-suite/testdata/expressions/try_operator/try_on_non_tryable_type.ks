// test: diagnostics
// stdlib: false
// include: try_prelude.ks

module Test
struct NotTryable {
    var value: lang.i64
}
func test() -> lang.i64 {
    let n = NotTryable(value: 42);
    try n // ERROR:
}
