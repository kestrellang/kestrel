// test: diagnostics
// stdlib: false
module Test

protocol Comparable { }
protocol Iterator {
    type Item;
}
protocol SortedIterator: Iterator where Iterator.Item: Comparable {
    func min() -> Item
}
