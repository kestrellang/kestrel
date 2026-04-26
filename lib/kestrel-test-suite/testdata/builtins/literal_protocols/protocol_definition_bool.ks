// test: diagnostics
// stdlib: false

module Test
@builtin(.ExpressibleByBoolLiteral)
protocol ExpressibleByBoolLiteral {
    init(boolLiteral value: lang.i1)
}
