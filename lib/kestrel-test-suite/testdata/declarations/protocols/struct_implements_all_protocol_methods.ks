// test: diagnostics
// stdlib: false
module Test
protocol Comparable {
    func lessThan(other: lang.i64) -> lang.i1
    func isEqual(to other: lang.i64) -> lang.i1
}
struct Number: Comparable {
    func lessThan(other: lang.i64) -> lang.i1 { true }
    func isEqual(to other: lang.i64) -> lang.i1 { false }
}
