// test: diagnostics
// stdlib: false

module Test
struct Bits: Prelude.ShiftRightOperatorProtocol {
    var value: lang.i64
    func shiftRight(rhs: Bits) -> Bits {
        Bits(value: lang.i64_signed_shr(self.value, rhs.value))
    }
}
func test() -> Bits {
    let a = Bits(value: 16);
    let b = Bits(value: 2);
    a >> b
}
