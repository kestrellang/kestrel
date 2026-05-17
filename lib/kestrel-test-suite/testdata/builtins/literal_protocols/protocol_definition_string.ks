// test: diagnostics
// stdlib: false

module Test
@builtin(.ExpressibleByStringLiteral)
protocol ExpressibleByStringLiteral {
    init(stringLiteral value: lang.str)
}
