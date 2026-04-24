// test: diagnostics
// stdlib: false
module Test

protocol Comparable {
    func compare(other: lang.i64) -> lang.i1
}
struct Number: Comparable {
    func compare(other: lang.i64) -> lang.i1 { true }
}
