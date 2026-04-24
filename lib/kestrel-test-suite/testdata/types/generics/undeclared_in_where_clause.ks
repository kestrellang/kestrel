// test: diagnostics
// stdlib: false

module Test

protocol Equatable { }
struct Set[T] where U: Equatable { } // ERROR: undeclared type parameter
