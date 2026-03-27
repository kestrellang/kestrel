// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.MultiplyOperatorProtocol {
    var value: lang.i64
    func multiply(rhs: Number) -> Number {
        Number(value: lang.i64_mul(self.value, rhs.value))
    }
}
func test() -> Number {
    let a = Number(value: 3);
    let b = Number(value: 4);
    a * b
}
