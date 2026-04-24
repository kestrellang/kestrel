// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
protocol BidirectionalIterator: Iterator {
    func prev() -> Item
}
