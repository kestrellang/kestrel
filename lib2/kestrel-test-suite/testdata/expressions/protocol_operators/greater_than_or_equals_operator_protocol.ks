// test: diagnostics
// stdlib: false

module Test
struct Number: Prelude.GreaterOrEqualOperatorProtocol {
    var value: lang.i64
    func greaterThanOrEqual(rhs: Number) -> lang.i1 {
        lang.i64_signed_ge(self.value, rhs.value)
    }
}
func test() -> lang.i1 {
    let a = Number(value: 5);
    let b = Number(value: 3);
    a >= b
}
