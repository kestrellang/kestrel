// test: diagnostics
// stdlib: false
module Test
protocol Comparable { func lessThan(other: Self) -> lang.i1 }
protocol Hashable { func hash() -> lang.i64 }
struct SortedBox[T] where T: Comparable { var value: T }
extend SortedBox[T] where T: Hashable {
    func getHash() -> lang.i64 { return self.value.hash(); }
}
