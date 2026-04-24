// test: diagnostics
// stdlib: false

module Test

protocol Iterator {
    type Item;
    func next() -> Item
}
