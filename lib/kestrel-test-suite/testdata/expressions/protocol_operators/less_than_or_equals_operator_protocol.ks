// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.LessOrEqualOperatorProtocol {
    var value: lang.i64
    func lessThanOrEqual(rhs: Number) -> lang.i1 {
        lang.i64_signed_le(self.value, rhs.value)
    }
}
func test() -> lang.i1 {
    let a = Number(value: 3);
    let b = Number(value: 5);
    a <= b
}
