// test: diagnostics
// stdlib: false

module Test
@builtin(.Copable) // ERROR: unknown builtin '.Copable'
protocol Foo {}
