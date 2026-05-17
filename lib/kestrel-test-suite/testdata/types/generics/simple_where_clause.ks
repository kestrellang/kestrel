// test: diagnostics
// stdlib: false

module Test

protocol Hashable {}
struct Container[T] where T: Hashable {}
