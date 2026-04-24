// test: diagnostics
// stdlib: false
module Test

protocol Comparable { }
protocol Iterator {
    type Item;
}
protocol SortedIterator: Iterator where Iterator.Item: Comparable { }
struct NotComparable { }
struct BadIterator: SortedIterator { // ERROR: does not satisfy bound
    type Item = NotComparable;
}
