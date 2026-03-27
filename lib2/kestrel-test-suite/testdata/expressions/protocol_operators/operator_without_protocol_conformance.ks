// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number {
    var value: lang.i64
}
func test() -> Number {
    let a = Number(value: 1);
    let b = Number(value: 2);
    a + b // ERROR: add
}
