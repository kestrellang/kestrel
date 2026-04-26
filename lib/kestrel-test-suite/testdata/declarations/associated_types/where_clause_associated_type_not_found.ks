// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
func process[T](iter: T) where T: Iterator, T.Unknown: Equatable { } // ERROR: no associated type 'Unknown'
