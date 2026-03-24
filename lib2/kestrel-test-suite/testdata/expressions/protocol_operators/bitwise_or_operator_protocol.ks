// test: diagnostics
// stdlib: false

module Test
struct Bits: Prelude.BitwiseOrOperatorProtocol {
    var value: lang.i64
    func bitwiseOr(rhs: Bits) -> Bits {
        Bits(value: lang.i64_or(self.value, rhs.value))
    }
}
func test() -> Bits {
    let a = Bits(value: 0b1100);
    let b = Bits(value: 0b1010);
    a | b
}
