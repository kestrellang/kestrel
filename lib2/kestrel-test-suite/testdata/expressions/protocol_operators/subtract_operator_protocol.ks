// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.SubtractOperatorProtocol {
    var value: lang.i64
    func subtract(rhs: Number) -> Number {
        Number(value: lang.i64_sub(self.value, rhs.value))
    }
}
func test() -> Number {
    let a = Number(value: 5);
    let b = Number(value: 3);
    a - b
}
