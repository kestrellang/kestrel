// test: diagnostics
// stdlib: false

module Test
@builtin(.Copyable)
protocol Copyable {}

@builtin(.ExpressibleByIntLiteral)
protocol ExpressibleByIntLiteral {
    init(intLiteral value: lang.i64)
}
