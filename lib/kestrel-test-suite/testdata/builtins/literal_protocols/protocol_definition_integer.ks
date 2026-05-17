// test: diagnostics
// stdlib: false

module Test
@builtin(.ExpressibleByIntLiteral)
protocol ExpressibleByIntegerLiteral {
    init(intLiteral value: lang.i64)
}
