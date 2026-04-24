// test: diagnostics
// stdlib: false
module Test

protocol Comparable {
    func compare(other: Self) -> lang.i1
}
struct Number: Comparable {
    func compare(other: Number) -> lang.i1 { true }
}
