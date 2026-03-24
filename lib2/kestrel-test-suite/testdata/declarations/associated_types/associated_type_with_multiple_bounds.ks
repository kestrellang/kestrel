// test: diagnostics
// stdlib: false

module Test

protocol Equatable { }
protocol Hashable { }
protocol Container {
    type Item: Equatable, Hashable
}
