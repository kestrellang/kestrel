// test: diagnostics
// stdlib: false

module Test
func take(consuming n: lang.i64) -> lang.i64 {
    lang.i64_mul(n, 2)
}
func test() -> lang.i64 {
    let x = 5;
    take(x)
}
