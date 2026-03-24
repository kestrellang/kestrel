// test: diagnostics
// stdlib: false

module Test
struct Bits: Prelude.BitwiseXorOperatorProtocol {
    var value: lang.i64
    func bitwiseXor(rhs: Bits) -> Bits {
        Bits(value: lang.i64_xor(self.value, rhs.value))
    }
}
func test() -> Bits {
    let a = Bits(value: 0b1100);
    let b = Bits(value: 0b1010);
    a ^ b
}
