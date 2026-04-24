// test: diagnostics
// stdlib: false
module Test
protocol Comparable {
    func compare(other: lang.i64) -> lang.i1
}
struct Number: Comparable { // ERROR: does not implement method 'compare'
    func compare() -> lang.i1 { }
}
