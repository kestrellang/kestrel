// test: diagnostics
// stdlib: false
module Test

protocol Container {
    type Element
    func add(item: Element)
}
extend Container {
    func addTwo(first: Element, second: Element) {
        self.add(first);
        self.add(second);
    }
}
