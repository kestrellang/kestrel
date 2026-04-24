// test: diagnostics
// stdlib: false

module Test
@builtin(.ExpressibleByIntLiteral)
protocol ExpressibleByIntLiteral {
    init(intLiteral value: lang.i64)
}
