// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
// -1 = unknown, 0 = false, 1 = true
struct Tristate: Prelude.ExpressibleByBoolLiteral {
    var state: lang.i64

    init(boolLiteral value: lang.i1) {
        if value {
            self.state = 1
        } else {
            self.state = 0
        }
    }
}
func test() {
    let yes: Tristate = true;
    let no: Tristate = false;
}
