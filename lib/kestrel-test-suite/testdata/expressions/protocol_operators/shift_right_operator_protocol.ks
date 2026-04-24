// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Bits: Prelude.ShiftRightOperatorProtocol {
    var value: lang.i64
    func shiftRight(by count: Bits) -> Bits {
        Bits(value: lang.i64_signed_shr(self.value, count.value))
    }
}
func test() -> Bits {
    let a = Bits(value: 16);
    let b = Bits(value: 2);
    a >> b
}
