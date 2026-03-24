// test: diagnostics
// stdlib: false

module Test
struct Bits: Prelude.BitwiseNotOperatorProtocol {
    var value: lang.i64
    func bitwiseNot() -> Bits {
        Bits(value: lang.i64_not(self.value))
    }
}
func test() -> Bits {
    let a = Bits(value: 0b1010);
    ~a
}
