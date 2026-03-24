// test: diagnostics
// stdlib: false

module Test
struct Number: Prelude.AddOperatorProtocol {
    var value: lang.i64
    func add(rhs: Number) -> Number {
        Number(value: lang.i64_add(self.value, rhs.value))
    }
}
func test() -> Number {
    let a = Number(value: 1);
    let b = Number(value: 2);
    let c = Number(value: 3);
    a + b + c
}
