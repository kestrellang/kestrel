// test: diagnostics
// stdlib: false
module Test

protocol Equatable { }
protocol Hashable { }
protocol Iterator {
    type Item;
}
func process[T](iter: T) where T: Iterator, T.Item: Equatable, T.Item: Hashable { }
