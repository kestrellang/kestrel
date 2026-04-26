// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
    func next() -> Item
}
protocol Iterable {
    type Item;
    type Iter: Iterator where Iter.Item = Item
    func iter() -> Iter
}
func useIter[I, T](iterable: I) -> T where I: Iterable, I.Item = T {
    let iter = iterable.iter();
    let item: T = iter.next();
    item
}
