// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.AddOperatorProtocol, Prelude.SubtractOperatorProtocol, Prelude.EqualsOperatorProtocol {
    var value: lang.i64
    func add(rhs: Number) -> Number {
        Number(value: lang.i64_add(self.value, rhs.value))
    }
    func subtract(rhs: Number) -> Number {
        Number(value: lang.i64_sub(self.value, rhs.value))
    }
    func isEqual(to rhs: Number) -> lang.i1 {
        lang.i64_eq(self.value, rhs.value)
    }
}
func test() -> lang.i1 {
    let a = Number(value: 5);
    let b = Number(value: 3);
    let c = Number(value: 2);
    (a - b) == c
}
