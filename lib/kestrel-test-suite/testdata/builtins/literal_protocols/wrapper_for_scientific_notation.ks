// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
struct Distance: Prelude.ExpressibleByFloatLiteral {
    var meters: lang.f64

    init(floatLiteral value: lang.f64) {
        self.meters = value
    }
}
func test() -> Distance {
    1.5e3
}
