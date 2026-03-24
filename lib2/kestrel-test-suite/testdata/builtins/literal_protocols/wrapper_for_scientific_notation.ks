// test: diagnostics
// stdlib: false

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
