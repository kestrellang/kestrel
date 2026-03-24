// test: diagnostics
// stdlib: false

module Test
struct Flag: Prelude.BooleanConditional {
    var value: lang.i1
    func asBool() -> lang.i1 { self.value }
}
func test(a: Flag, b: Flag) -> lang.i64 {
    if lang.i1_or(a.asBool(), b.asBool()) {
        1
    } else {
        0
    }
}
