// test: diagnostics
// stdlib: false

module Test

protocol Equatable { }
protocol Container[T] { }
struct Set[T]: Container[T] where T: Equatable { }
