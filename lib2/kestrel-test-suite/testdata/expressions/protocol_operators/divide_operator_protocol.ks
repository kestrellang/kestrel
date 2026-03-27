// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.DivideOperatorProtocol {
    var value: lang.i64
    func divide(rhs: Number) -> Number {
        Number(value: lang.i64_signed_div(self.value, rhs.value))
    }
}
func test() -> Number {
    let a = Number(value: 10);
    let b = Number(value: 2);
    a / b
}
