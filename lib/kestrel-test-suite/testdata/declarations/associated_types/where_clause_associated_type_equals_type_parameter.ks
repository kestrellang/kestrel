// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
    func next() -> Item
}
func collect[T, U](iter: T) -> U where T: Iterator, T.Item = U {
    iter.next()
}
