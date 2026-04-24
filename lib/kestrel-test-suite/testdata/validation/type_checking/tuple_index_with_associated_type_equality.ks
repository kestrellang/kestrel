// test: diagnostics
// stdlib: false

module Main

protocol Iterable {
    type Item
}

extend Iterable {
    func split[A, B](pair: Item) -> (A, B) where Item = (A, B) {
        let first: A = pair.0;
        let second: B = pair.1;
        return (first, second);
    }
}
