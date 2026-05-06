// test: diagnostics
// stdlib: false
module Test

protocol Comparable {
    func compare(other: Self)
}
protocol Equatable {
    func isEqual(to other: Self)
}
protocol Iterator {
    type Item
    func next()
}
extend Iterator where Self: Comparable, Self.Item: Equatable {
    func mixedHelper() { }
}
