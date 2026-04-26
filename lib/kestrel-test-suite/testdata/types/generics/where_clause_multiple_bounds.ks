// test: diagnostics
// stdlib: false

module Test

protocol Hashable {}
protocol Equatable {}
struct Set[T] where T: Hashable, T: Equatable {}
