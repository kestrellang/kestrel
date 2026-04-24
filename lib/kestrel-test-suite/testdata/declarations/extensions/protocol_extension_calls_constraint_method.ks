// test: diagnostics
// stdlib: false
module Test

protocol Sortable {
    func sort()
}
protocol Filterable {
    func filter()
}
extend Filterable where Self: Sortable {
    func filterAndSort() {
        self.filter();
        self.sort();
    }
}
struct Data: Filterable, Sortable {
    func filter() { }
    func sort() { }
}
func test() {
    let d = Data();
    d.filterAndSort();
}
