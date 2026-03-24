// test: diagnostics
// stdlib: false

module Test

protocol Foo {}
protocol Foo {} // ERROR: duplicate
