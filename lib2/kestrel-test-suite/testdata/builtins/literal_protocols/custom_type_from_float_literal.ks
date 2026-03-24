// test: diagnostics
// stdlib: false

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
