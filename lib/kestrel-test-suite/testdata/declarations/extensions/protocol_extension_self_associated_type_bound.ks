// test: diagnostics
// stdlib: false
module Test

protocol Equatable {
    func equals(other: Self)
}
protocol Iterator {
    type Item
    func next()
}
extend Iterator where Self.Item: Equatable {
    func containsHelper() { }
}
