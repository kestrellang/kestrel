// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
struct Flag: Prelude.ExpressibleByBoolLiteral {
    var enabled: lang.i1

    init(boolLiteral value: lang.i1) {
        self.enabled = value
    }
}
func test() -> Flag {
    true
}
