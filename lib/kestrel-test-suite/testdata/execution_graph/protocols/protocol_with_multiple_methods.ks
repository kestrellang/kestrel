// test: diagnostics
// stdlib: false

module Test

protocol Comparable {
    func lessThan(other: lang.i64) -> lang.i1
    func equals(other: lang.i64) -> lang.i1
}
