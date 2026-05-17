// test: diagnostics
// stdlib: true
module Test

protocol Collection {
    func getAll() -> [Self]
}
struct Item: Collection {
    func getAll() -> [Item] { [] }
}
