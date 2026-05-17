// test: diagnostics
// stdlib: false
module Test

protocol Equatable {
    func isEqual(to other: Self)
}
protocol Iterator {
    type Item
    func next()
}
extend Iterator where Self.Item: Equatable {
    func containsHelper() { }
}
