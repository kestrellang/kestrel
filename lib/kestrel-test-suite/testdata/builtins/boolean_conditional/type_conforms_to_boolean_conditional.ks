// test: diagnostics
// stdlib: true

module Test
struct Flag: BooleanConditional {
    var enabled: lang.i1

    func boolValue() -> lang.i1 {
        self.enabled
    }
}
