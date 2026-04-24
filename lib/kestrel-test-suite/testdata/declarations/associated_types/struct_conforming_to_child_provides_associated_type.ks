// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
    func next() -> Item
}
protocol BidirectionalIterator: Iterator {
    func prev() -> Item
}
struct IntBiIterator: Iterator, BidirectionalIterator {
    type Item = lang.i64;
    func next() -> lang.i64 { 0 }
    func prev() -> lang.i64 { 0 }
}
