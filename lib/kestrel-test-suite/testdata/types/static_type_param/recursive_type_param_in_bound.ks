// test: diagnostics
// stdlib: false

module Test

protocol Comparable[Other] {
    func compare(other: Other) -> lang.i64
}
func compareToSelf[T](a: T, b: T) -> lang.i64 where T: Comparable[T] {
    a.compare(b)
}
