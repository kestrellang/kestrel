// test: diagnostics
// stdlib: false

module Test
@builtin(.ExpressibleByFloatLiteral)
protocol ExpressibleByFloatLiteral {
    init(floatLiteral value: lang.f64)
}
