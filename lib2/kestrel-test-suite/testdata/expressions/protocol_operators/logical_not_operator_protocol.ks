// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Flag: Prelude.LogicalNotOperatorProtocol {
    var value: lang.i1
    func logicalNot() -> lang.i1 {
        lang.i1_not(self.value)
    }
}
func test() -> lang.i1 {
    let f = Flag(value: true);
    not f
}
