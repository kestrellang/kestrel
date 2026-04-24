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
struct Data: Filterable {
    func filter() { }
}
func test() {
    let d = Data();
    d.combined(); // ERROR: member
}
