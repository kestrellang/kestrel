// test: diagnostics
// stdlib: false

module Test

struct Foo {}
protocol Foo {} // ERROR: duplicate
