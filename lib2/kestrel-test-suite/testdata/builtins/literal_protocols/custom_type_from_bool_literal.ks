// test: diagnostics
// stdlib: false

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
