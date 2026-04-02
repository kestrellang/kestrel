// test: diagnostics
// stdlib: true

module Test
struct Flag: BooleanConditional {
    var value: lang.i1
    func boolValue() -> lang.i1 { self.value }
}
func test(a: Flag, b: Flag) -> lang.i64 {
    if lang.i1_or(a.boolValue(), b.boolValue()) {
        1
    } else {
        0
    }
}
