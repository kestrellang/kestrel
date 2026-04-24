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
    func combined() { }
}
struct Data: Filterable, Sortable {
    func filter() { }
    func sort() { }
}
func test() {
    let d = Data();
    d.combined();
}
