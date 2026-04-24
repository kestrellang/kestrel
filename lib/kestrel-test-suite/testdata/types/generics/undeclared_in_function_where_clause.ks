// test: diagnostics
// stdlib: false

module Test

protocol Comparable { }
func sort[T](items: T) where U: Comparable { } // ERROR: undeclared type parameter
