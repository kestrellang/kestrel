// test: diagnostics
// stdlib: false
module Test

protocol Sortable {
    func sort()
}
protocol Filterable {
    func filter()
}
// Less constrained extension (specificity 0)
extend Filterable {
    func process() { }
}
// More constrained extension (specificity 1) - doesn't apply to BasicData
extend Filterable where Self: Sortable {
    func process() { }
}
// BasicData only conforms to Filterable, not Sortable
struct BasicData: Filterable {
    func filter() { }
}
func test() {
    let d = BasicData();
    d.process();
}
