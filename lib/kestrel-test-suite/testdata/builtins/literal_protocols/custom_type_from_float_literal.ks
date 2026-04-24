// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
struct Temperature: Prelude.ExpressibleByFloatLiteral {
    var celsius: lang.f64

    init(floatLiteral value: lang.f64) {
        self.celsius = value
    }
}
func test() -> Temperature {
    36.6
}
