// test: diagnostics
// stdlib: false

module Test
@builtin(feature: .Copyable) // ERROR: '@builtin' has no argument 'feature'
protocol Foo {}
