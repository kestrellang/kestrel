// test: diagnostics
// stdlib: false

module Test

protocol Equatable {
    func isEqual(to other: Self) -> lang.i1
}
protocol Comparable: Equatable {
    func lessThan(other: Self) -> lang.i1
}
func checkEqual[T](a: T, b: T) -> lang.i1 where T: Comparable {
    return a.isEqual(to: b)
}
