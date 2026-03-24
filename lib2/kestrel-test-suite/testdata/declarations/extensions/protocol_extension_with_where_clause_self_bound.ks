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
    func helper() { }
}
