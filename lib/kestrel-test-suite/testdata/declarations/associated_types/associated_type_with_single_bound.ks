// test: diagnostics
// stdlib: false

module Test

protocol Equatable { }
protocol Container {
    type Item: Equatable
}
