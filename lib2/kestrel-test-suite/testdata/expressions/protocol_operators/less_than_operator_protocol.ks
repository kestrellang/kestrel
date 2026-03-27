// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.LessThanOperatorProtocol {
    var value: lang.i64
    func lessThan(rhs: Number) -> lang.i1 {
        lang.i64_signed_lt(self.value, rhs.value)
    }
}
func test() -> lang.i1 {
    let a = Number(value: 3);
    let b = Number(value: 5);
    a < b
}
