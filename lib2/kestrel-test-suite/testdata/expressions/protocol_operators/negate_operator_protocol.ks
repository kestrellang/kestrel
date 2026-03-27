// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.NegateOperatorProtocol {
    var value: lang.i64
    func negate() -> Number {
        Number(value: lang.i64_neg(self.value))
    }
}
func test() -> Number {
    let a = Number(value: 5);
    -a
}
