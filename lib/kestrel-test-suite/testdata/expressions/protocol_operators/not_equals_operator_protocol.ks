// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Number: Prelude.NotEqualsOperatorProtocol {
    var value: lang.i64
    func notEqual(to rhs: Number) -> lang.i1 {
        lang.i64_ne(self.value, rhs.value)
    }
}
func test() -> lang.i1 {
    let a = Number(value: 5);
    let b = Number(value: 3);
    a != b
}
