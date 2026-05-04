// test: diagnostics
// stdlib: false

module Test

protocol Comparable {
    func lessThan(other: Self) -> lang.i1
    func greaterThan(other: Self) -> lang.i1
    func isEqual(to other: Self) -> lang.i1
}
