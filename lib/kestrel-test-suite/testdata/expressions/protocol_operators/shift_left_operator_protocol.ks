// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Bits: Prelude.ShiftLeftOperatorProtocol {
    var value: lang.i64
    func shiftLeft(by count: Bits) -> Bits {
        Bits(value: lang.i64_shl(self.value, count.value))
    }
}
func test() -> Bits {
    let a = Bits(value: 1);
    let b = Bits(value: 4);
    a << b
}
