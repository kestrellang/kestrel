// test: diagnostics
// stdlib: false

module Test

protocol Comparable[U] { }
struct Collection[T] where T: Comparable[T] { }
