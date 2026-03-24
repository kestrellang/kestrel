// test: diagnostics
// stdlib: false

module Test
struct Bits: Prelude.ShiftLeftOperatorProtocol {
    var value: lang.i64
    func shiftLeft(rhs: Bits) -> Bits {
        Bits(value: lang.i64_shl(self.value, rhs.value))
    }
}
func test() -> Bits {
    let a = Bits(value: 1);
    let b = Bits(value: 4);
    a << b
}
