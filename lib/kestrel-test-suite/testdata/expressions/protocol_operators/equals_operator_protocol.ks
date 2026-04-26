// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.EqualsOperatorProtocol {
    var value: lang.i64
    func equals(rhs: Number) -> lang.i1 {
        lang.i64_eq(self.value, rhs.value)
    }
}
func test() -> lang.i1 {
    let a = Number(value: 5);
    let b = Number(value: 5);
    a == b
}
