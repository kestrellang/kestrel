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
// More constrained extension (specificity 1)
extend Filterable where Self: Sortable {
    func process() { }
}
struct Data: Filterable, Sortable {
    func filter() { }
    func sort() { }
}
func test() {
    let d = Data();
    d.process();
}
