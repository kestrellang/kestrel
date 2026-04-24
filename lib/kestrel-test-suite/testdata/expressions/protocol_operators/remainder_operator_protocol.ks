// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.ModuloOperatorProtocol {
    var value: lang.i64
    func modulo(rhs: Number) -> Number {
        Number(value: lang.i64_signed_rem(self.value, rhs.value))
    }
}
func test() -> Number {
    let a = Number(value: 10);
    let b = Number(value: 3);
    a % b
}
