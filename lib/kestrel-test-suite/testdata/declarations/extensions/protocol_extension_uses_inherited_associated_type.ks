// test: diagnostics
// stdlib: false
module Test

protocol Base {
    type Element
}
protocol Child: Base {
    func fetch() -> Element
}
extend Child {
    func fetchWithFallback(fallback: Element) -> Element {
        return self.fetch();
    }
}
