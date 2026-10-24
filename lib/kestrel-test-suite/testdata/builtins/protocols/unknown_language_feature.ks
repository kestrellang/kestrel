// test: diagnostics
// stdlib: false

module Test
@builtin(.UnknownFeature) // ERROR: unknown builtin '.UnknownFeature'
protocol Foo {}
