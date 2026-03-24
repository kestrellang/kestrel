// test: diagnostics
// stdlib: false

module Test
struct Number {
    var value: lang.i64
}
func test() -> Number {
    let a = Number(value: 1);
    let b = Number(value: 2);
    a + b // ERROR: add
}
