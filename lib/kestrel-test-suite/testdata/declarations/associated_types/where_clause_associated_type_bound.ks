// test: diagnostics
// stdlib: false
module Test

protocol Equatable { }
protocol Iterator {
    type Item;
}
func findEqual[T](iter: T) where T: Iterator, T.Item: Equatable { }
