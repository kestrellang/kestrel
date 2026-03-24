// test: diagnostics
// stdlib: false

module Test

protocol Foo: Foo {} // ERROR
