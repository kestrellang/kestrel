// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct NumberA: Prelude.AddOperatorProtocol {
    var value: lang.i64
    func add(rhs: NumberA) -> NumberA {
        NumberA(value: lang.i64_add(self.value, rhs.value))
    }
}
struct NumberB {
    var value: lang.i64
}
func test() {
    let a = NumberA(value: 1);
    let b = NumberB(value: 2);
    a + b // ERROR:
}
