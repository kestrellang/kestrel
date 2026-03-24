// test: diagnostics
// stdlib: false

module Test
struct Flag: Prelude.BooleanConditional {
    var enabled: lang.i1

    func asBool() -> lang.i1 {
        self.enabled
    }
}
