// test: diagnostics
// stdlib: false
module Test
protocol Comparable { func lessThan(other: Self) -> lang.i1 }
struct SortedBox[T] where T: Comparable { var value: T }
extend SortedBox[T] {
    func isLessThan(other: SortedBox[T]) -> lang.i1 { return self.value.lessThan(other.value); }
}
