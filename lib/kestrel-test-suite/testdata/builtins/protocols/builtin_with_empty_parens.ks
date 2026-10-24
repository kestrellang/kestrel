// test: diagnostics
// stdlib: false

module Test
@builtin() // ERROR: '@builtin' needs a builtin name: '@builtin(.Copyable)'
protocol Foo {}
