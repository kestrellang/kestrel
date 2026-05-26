// test: diagnostics
// stdlib: false
module Test
protocol Comparable {
    func lessThan(other: lang.i64) -> lang.i1
    func isEqual(to other: lang.i64) -> lang.i1
}
struct Number: Comparable { // ERROR: does not implement method 'equals'
    func lessThan(other: lang.i64) -> lang.i1 { }
}
