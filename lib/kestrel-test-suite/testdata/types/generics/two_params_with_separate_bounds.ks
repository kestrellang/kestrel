// test: diagnostics
// stdlib: false

module Test

protocol Equatable { }
protocol Hashable { }
struct BiMap[K, V] where K: Equatable, V: Hashable { }
